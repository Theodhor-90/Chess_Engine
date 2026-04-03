use criterion::{black_box, criterion_group, criterion_main, Criterion};

use chess_board::Position;
use chess_nnue::{forward, Accumulator, AccumulatorStack, Network};
use chess_types::Color;

fn make_test_network() -> Network {
    let mut net = Network::new_zeroed();
    for (i, w) in net.input_weights.iter_mut().enumerate() {
        *w = ((i % 256) as i16).wrapping_sub(128);
    }
    for (i, b) in net.input_bias.iter_mut().enumerate() {
        *b = (i as i16) % 50 - 25;
    }
    for (i, w) in net.hidden1_weights.iter_mut().enumerate() {
        *w = ((i % 128) as i8).wrapping_sub(64);
    }
    for (i, b) in net.hidden1_bias.iter_mut().enumerate() {
        *b = (i as i32) * 100 - 1600;
    }
    for (i, w) in net.hidden2_weights.iter_mut().enumerate() {
        *w = ((i % 64) as i8).wrapping_sub(32);
    }
    net.hidden2_bias = 42;
    net
}

fn bench_forward_pass(c: &mut Criterion) {
    let net = make_test_network();
    let mut acc = Accumulator::new();
    acc.init_from_bias(&net.input_bias);
    // Add some features to make it non-trivial
    for idx in 0..20 {
        acc.add_feature(Color::White, idx * 100, &net.input_weights);
        acc.add_feature(Color::Black, idx * 100 + 50, &net.input_weights);
    }

    c.bench_function("forward_pass", |b| {
        b.iter(|| black_box(forward(black_box(&acc), black_box(&net), Color::White)))
    });
}

fn bench_accumulator_add_remove(c: &mut Criterion) {
    let net = make_test_network();
    let mut acc = Accumulator::new();
    acc.init_from_bias(&net.input_bias);

    let feature_indices: Vec<usize> = (0..16).map(|i| i * 200).collect();

    c.bench_function("accumulator_add_remove", |b| {
        b.iter(|| {
            for &idx in &feature_indices {
                acc.add_feature(Color::White, black_box(idx), &net.input_weights);
            }
            for &idx in &feature_indices {
                acc.remove_feature(Color::White, black_box(idx), &net.input_weights);
            }
        })
    });
}

fn bench_evaluate_position(c: &mut Criterion) {
    let net = make_test_network();
    let pos = Position::startpos();

    c.bench_function("evaluate_position", |b| {
        b.iter(|| {
            let mut stack = AccumulatorStack::new();
            stack.refresh_if_needed(black_box(&pos), black_box(&net));
            black_box(stack.evaluate(black_box(&pos), black_box(&net), Color::White))
        })
    });
}

fn bench_push_pop_cycle(c: &mut Criterion) {
    let net = make_test_network();
    let mut pos = Position::startpos();
    let mut stack = AccumulatorStack::new();
    stack.refresh_if_needed(&pos, &net);

    let moves = chess_movegen::generate_legal_moves(&mut pos);
    let mv = moves[0];

    // Pre-make the move to capture state
    let undo = pos.make_move(mv);
    pos.unmake_move(mv, undo);

    c.bench_function("push_pop_cycle", |b| {
        b.iter(|| {
            let undo = pos.make_move(mv);
            stack.push_move(
                black_box(&pos),
                black_box(mv),
                undo.captured,
                black_box(&net),
            );
            stack.pop();
            pos.unmake_move(mv, undo);
        })
    });
}

criterion_group!(
    benches,
    bench_forward_pass,
    bench_accumulator_add_remove,
    bench_evaluate_position,
    bench_push_pop_cycle,
);
criterion_main!(benches);
