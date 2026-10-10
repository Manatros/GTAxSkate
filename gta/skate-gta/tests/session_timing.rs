//! How long Skate's session takes to load, part by part (stderr), and the
//! memory a second one costs, from the real converted data.
#[test]
#[ignore = "reads the real Skate data"]
fn session_load_time() {
    let assets = std::path::Path::new(r"C:\Games\2010-Rust-Rewrite-Mashup\skate-data\assets");
    let floor = || vec![[[-50., 0., -50.], [50., 0., -50.], [0., 0., 50.]]];
    let started = std::time::Instant::now();
    let first = skate_host::bridge::Session::new(assets, floor(), vec![], [0., 0., 0.], 0.).unwrap();
    eprintln!("first {} ms", started.elapsed().as_millis());
    let started = std::time::Instant::now();
    let second = skate_host::bridge::Session::new(assets, floor(), vec![], [0., 0., 0.], 0.).unwrap();
    eprintln!("second {} ms", started.elapsed().as_millis());
    std::thread::sleep(std::time::Duration::from_secs(20));
    drop((first, second));
}