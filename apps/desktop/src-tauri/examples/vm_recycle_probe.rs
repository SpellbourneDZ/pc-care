fn main() {
    match pc_care::win::adapters::recycle_probe::run() {
        Ok(report) => println!("{report}"),
        Err(error) => {
            eprintln!("VM probe refused/failed: {error}");
            std::process::exit(1);
        }
    }
}
