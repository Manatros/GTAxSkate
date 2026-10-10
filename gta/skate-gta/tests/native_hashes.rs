//! Every native the plugin calls is named by its full 64-bit hash: a short
//! (old-style) one is not found by Script Hook V, which then stops GTA
//! ("Can't find native 0x...ECB2FDEA").
#[test]
fn every_native_hash_is_a_full_one() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut bad = Vec::new();
    let mut files = vec![dir.clone(), dir.join("win")];
    while let Some(d) = files.pop() {
        for entry in std::fs::read_dir(&d).unwrap().flatten() {
            let path = entry.path();
            if path.extension().is_some_and(|e| e == "rs") {
                let text = std::fs::read_to_string(&path).unwrap();
                for (at, _) in text.match_indices("native!(") {
                    let rest = text[at + 8..].trim_start();
                    if let Some(hex) = rest.strip_prefix("0x") {
                        let digits = hex.chars().take_while(|c| c.is_ascii_hexdigit()).count();
                        if digits != 16 {
                            bad.push(format!("{}: 0x{}", path.display(), &hex[..digits]));
                        }
                    }
                }
            }
        }
    }
    assert!(bad.is_empty(), "short native hashes: {bad:?}");
}
