//! Prints the golden fixture to stdout. Stderr gets a timing line for a
//! 128×128 extract, which is not part of the byte comparison.

fn main() {
    let started = std::time::Instant::now();
    let image = homechameleon_core::gradient(128, 128);
    let request = homechameleon_core::GenerateRequest {
        name: "Bench".into(),
        images: vec![image],
        ..homechameleon_core::GenerateRequest::default()
    };
    let _ = homechameleon_core::generate(&request).expect("128px extract");
    eprintln!("extract_128_ms={}", started.elapsed().as_millis());
    print!("{}", homechameleon_core::fixture_report());
}
