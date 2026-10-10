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

/// Every native hash is one GTA has: checked against a copy of the public
/// native database (alloc8or/gta5-nativedb-data `natives.json`) in the
/// SkateMods `tools` folder, when it is there (a wrong hash stops GTA).
#[test]
fn every_native_hash_is_in_the_native_database() {
    let manifest = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let Some(db) = manifest.ancestors().map(|a| a.join("tools").join("natives.json")).find(|p| p.is_file()) else {
        eprintln!("no tools/natives.json: native hashes not checked against the database");
        return;
    };
    let db: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(db).unwrap()).unwrap();
    let known: std::collections::HashSet<String> = db
        .as_object()
        .unwrap()
        .values()
        .flat_map(|ns| ns.as_object().unwrap().keys().map(|h| h.to_ascii_uppercase()))
        .collect();
    let dir = manifest.join("src");
    let mut unknown = Vec::new();
    let mut files = vec![dir.clone(), dir.join("win")];
    while let Some(d) = files.pop() {
        for entry in std::fs::read_dir(&d).unwrap().flatten() {
            let path = entry.path();
            if path.extension().is_some_and(|e| e == "rs") {
                let text = std::fs::read_to_string(&path).unwrap();
                for (at, _) in text.match_indices("native!(0x") {
                    let hex: String = text[at + 10..].chars().take_while(|c| c.is_ascii_hexdigit()).collect();
                    if !known.contains(&format!("0X{}", hex.to_ascii_uppercase())) {
                        unknown.push(format!("{}: 0x{hex}", path.file_name().unwrap().to_string_lossy()));
                    }
                }
            }
        }
    }
    assert!(unknown.is_empty(), "natives GTA does not have: {unknown:?}");
}
