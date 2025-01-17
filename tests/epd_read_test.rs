use chess_actix::{
    benchmark::epd_reader::{self, EpdRead},
    ui::notation::san,
};
use san::Lang;

#[test]
fn test_epd_read_from_file() {
    let epd_file_path = "tests/data/example.epd";
    let epd_reader = epd_reader::EpdFile(epd_file_path.to_string());
    let lang = Lang::LangEn;

    let result = epd_reader.epd_read(&lang);
    assert!(result.is_ok());
    let epds = result.unwrap();

    // Validate the output
    assert_eq!(epds.len(), 2); // Example file has 2 lines
    assert_eq!(
        epds[0].to_string(),
        "rn1qkb1r/pp2pppp/5n2/3p1b2/3P4/2N1P3/PP3PPP/R1BQKBNR w KQkq - 0 1 id \"CCR01\";bm Qb3;"
    );
    assert_eq!(
        epds[1].to_string(),
        "rn1qkb1r/pp2pppp/5n2/3p1b2/3P4/1QN1P3/PP3PPP/R1B1KBNR b KQkq - 1 1 id \"CCR02\";bm Bc8;"
    );
}
