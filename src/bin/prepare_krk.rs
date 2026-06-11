use chess_actix::training::permutation::prepare_krk_as_fen_white_turn_to_csv;

pub fn main() {
    let file_name = "krk_position.csv";
    // 175_168 positions
    prepare_krk_as_fen_white_turn_to_csv(file_name)
        .expect("Failed to prepare KRK positions as FEN in CSV");
}
