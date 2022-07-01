fn main() {
    let techmino = block_stacker::Ruleset::techmino();
    let trs = blockfish::ShapeTable::from_ruleset(&techmino);
    let stdout = std::io::stdout();
    serde_json::to_writer(stdout.lock(), &trs).unwrap();
}
