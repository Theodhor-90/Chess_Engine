use bullet_lib::{
    game::inputs::SparseInputType,
    nn::optimiser,
    trainer::{
        save::SavedFormat,
        schedule::{lr, wdl, TrainingSchedule, TrainingSteps},
        settings::LocalSettings,
    },
    value::{loader, ValueTrainerBuilder},
};
use bulletformat::ChessBoard;
use std::env;

/// HalfKP input features: 64 king squares × 10 colored non-king piece types × 64 piece squares
/// = 40,960 features per perspective.
///
/// Feature index = king_sq * 640 + colored_piece_index * 64 + piece_sq
/// where colored_piece_index = piece_color * 5 + piece_kind
/// piece_kind: Pawn=0, Knight=1, Bishop=2, Rook=3, Queen=4
/// piece_color: 0=white, 1=black (relative to perspective in bulletformat)
const HALFKP_FEATURES: usize = 40_960;
const L1_SIZE: usize = 256;
const L2_SIZE: usize = 32;

const SCALE: i32 = 400;
const QA: i16 = 64; // 2^6, matching QUANTIZATION_SHIFT_ACCUMULATOR

#[derive(Clone, Copy, Debug)]
struct HalfKP;

impl SparseInputType for HalfKP {
    type RequiredDataType = ChessBoard;

    fn num_inputs(&self) -> usize {
        HALFKP_FEATURES
    }

    fn max_active(&self) -> usize {
        // At most 30 non-king pieces on the board
        30
    }

    fn map_features<F: FnMut(usize, usize)>(&self, pos: &Self::RequiredDataType, mut f: F) {
        let our_king = usize::from(pos.our_ksq());
        let opp_king = usize::from(pos.opp_ksq());

        for (piece, square) in pos.into_iter() {
            let pc_type = usize::from(piece & 7);
            let pc_color = usize::from(piece & 8 > 0);

            // Skip kings (piece type 5 in bulletformat)
            if pc_type == 5 {
                continue;
            }

            // Map bulletformat piece types to our piece kinds:
            // bulletformat: Pawn=0, Knight=1, Bishop=2, Rook=3, Queen=4, King=5
            // Our mapping: Pawn=0, Knight=1, Bishop=2, Rook=3, Queen=4
            let piece_kind = pc_type;
            let sq = usize::from(square);

            // STM (side-to-move) perspective feature
            let stm_colored_piece = [0, 5][pc_color] + piece_kind;
            let stm_idx = our_king * 640 + stm_colored_piece * 64 + sq;

            // NSTM (not-side-to-move) perspective feature
            // For the opponent's perspective, colors are swapped and squares are flipped
            let nstm_colored_piece = [5, 0][pc_color] + piece_kind;
            let nstm_idx = opp_king * 640 + nstm_colored_piece * 64 + (sq ^ 56);

            f(stm_idx, nstm_idx);
        }
    }

    fn shorthand(&self) -> String {
        "halfkp-40960".to_string()
    }

    fn description(&self) -> String {
        "HalfKP 40960 features (64 king sq × 10 colored piece types × 64 piece sq)".to_string()
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();

    let data_path = get_arg(&args, "--data").expect("--data <path> is required");
    let output_dir = get_arg(&args, "--output-dir").expect("--output-dir <path> is required");
    let epochs: usize = get_arg(&args, "--epochs")
        .map(|s| s.parse().expect("invalid --epochs"))
        .unwrap_or(100);
    let batch_size: usize = get_arg(&args, "--batch-size")
        .map(|s| s.parse().expect("invalid --batch-size"))
        .unwrap_or(16_384);
    let lr_val: f64 = get_arg(&args, "--lr")
        .map(|s| s.parse().expect("invalid --lr"))
        .unwrap_or(0.001);

    let mut trainer = ValueTrainerBuilder::default()
        .dual_perspective()
        .optimiser(optimiser::AdamW)
        .inputs(HalfKP)
        .save_format(&[
            SavedFormat::id("l0w").round().quantise::<i16>(QA),
            SavedFormat::id("l0b").round().quantise::<i16>(QA),
            SavedFormat::id("l1w").transpose().round().quantise::<i8>(QA),
            SavedFormat::id("l1b").round().quantise::<i32>(QA as i32 * QA as i32),
            SavedFormat::id("l2w").transpose().round().quantise::<i8>(QA),
            SavedFormat::id("l2b").round().quantise::<i32>(QA as i32 * QA as i32),
        ])
        .loss_fn(|output, target| output.sigmoid().squared_error(target))
        .build(|builder, stm_inputs, ntm_inputs| {
            let l0 = builder.new_affine("l0", HALFKP_FEATURES, L1_SIZE);
            let l1 = builder.new_affine("l1", 2 * L1_SIZE, L2_SIZE);
            let l2 = builder.new_affine("l2", L2_SIZE, 1);

            let stm_hidden = l0.forward(stm_inputs).crelu();
            let ntm_hidden = l0.forward(ntm_inputs).crelu();
            let hidden = stm_hidden.concat(ntm_hidden);
            let h2 = l1.forward(hidden).crelu();
            l2.forward(h2)
        });

    // LR schedule: drop by 0.1× at epochs 75 and 90
    // Using StepLR as an approximation — step every N superbatches
    let schedule = TrainingSchedule {
        net_id: "halfkp".to_string(),
        eval_scale: SCALE as f32,
        steps: TrainingSteps {
            batch_size,
            batches_per_superbatch: 6104,
            start_superbatch: 1,
            end_superbatch: epochs,
        },
        wdl_scheduler: wdl::ConstantWDL { value: 0.5 },
        lr_scheduler: lr::StepLR {
            start: lr_val as f32,
            gamma: 0.1,
            step: 75,
        },
        save_rate: 10,
    };

    let settings = LocalSettings {
        threads: 4,
        test_set: None,
        output_directory: &output_dir,
        batch_queue_size: 64,
    };

    let data_loader = loader::DirectSequentialDataLoader::new(&[&data_path]);

    trainer.run(&schedule, &settings, &data_loader);
}

fn get_arg(args: &[String], flag: &str) -> Option<String> {
    args.iter()
        .position(|a| a == flag)
        .and_then(|i| args.get(i + 1))
        .cloned()
}
