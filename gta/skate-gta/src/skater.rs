//! Your own Skate 3 skater, the fourth character: the catalog of
//! Create-a-Skater parts `skate-ped` built into the add-on ped
//! (`SkateGTA-skater.json`), the outfit you put together from it
//! (`SkateGTA-skater.txt`), and the Skate-style editor's rows.

use std::collections::BTreeMap;

use serde::Deserialize;

#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct Drawable {
    pub name: String,
    #[serde(default)]
    pub textures: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct Component {
    /// GTA's ped component slot, 0 (head) to 11 (top).
    pub component: u8,
    pub label: String,
    #[serde(default)]
    pub default: usize,
    pub drawables: Vec<Drawable>,
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct Catalog {
    pub ped: String,
    pub components: Vec<Component>,
}

impl Catalog {
    pub fn from_json(text: &str) -> Result<Self, String> {
        let catalog: Self = serde_json::from_str(text).map_err(|e| format!("skater catalog: {e}"))?;
        if catalog.ped.is_empty() || catalog.components.iter().any(|c| c.component > 11 || c.drawables.is_empty()) {
            return Err("skater catalog: a component is broken".into());
        }
        Ok(catalog)
    }
}

/// The part and finish worn in each component.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Outfit {
    pub worn: BTreeMap<u8, (usize, usize)>,
}

impl Outfit {
    /// The default skater's parts.
    pub fn default_for(catalog: &Catalog) -> Self {
        let worn = catalog
            .components
            .iter()
            .map(|c| (c.component, (c.default.min(c.drawables.len() - 1), 0)))
            .collect();
        Self { worn }
    }

    /// One `component drawable texture` line each.
    pub fn to_text(&self) -> String {
        self.worn.iter().map(|(c, (d, t))| format!("{c} {d} {t}\n")).collect()
    }

    /// Reads [`to_text`](Self::to_text) against the catalog: unknown or
    /// out-of-range lines keep the default.
    pub fn from_text(text: &str, catalog: &Catalog) -> Self {
        let mut outfit = Self::default_for(catalog);
        for line in text.lines() {
            let numbers: Vec<usize> = line.split_whitespace().filter_map(|w| w.parse().ok()).collect();
            let [c, d, t] = numbers[..] else { continue };
            let Some(component) = catalog.components.iter().find(|x| usize::from(x.component) == c) else { continue };
            let Some(drawable) = component.drawables.get(d) else { continue };
            if t < drawable.textures.len().max(1) {
                outfit.worn.insert(component.component, (d, t));
            }
        }
        outfit
    }
}

/// The editor's rows: each component's part, then its finish.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Row {
    Part(usize),
    Finish(usize),
    Random,
    Reset,
}

pub fn rows(catalog: &Catalog) -> Vec<Row> {
    let mut rows = Vec::new();
    for i in 0..catalog.components.len() {
        rows.push(Row::Part(i));
        rows.push(Row::Finish(i));
    }
    rows.push(Row::Random);
    rows.push(Row::Reset);
    rows
}

/// A row's label and value.
pub fn row_text(row: Row, catalog: &Catalog, outfit: &Outfit) -> (String, String) {
    let worn = |i: usize| {
        let c = &catalog.components[i];
        let (d, t) = outfit.worn.get(&c.component).copied().unwrap_or((c.default, 0));
        (c, d.min(c.drawables.len() - 1), t)
    };
    match row {
        Row::Part(i) => {
            let (c, d, _) = worn(i);
            (c.label.clone(), format!("{} ({}/{})", c.drawables[d].name, d + 1, c.drawables.len()))
        }
        Row::Finish(i) => {
            let (c, d, t) = worn(i);
            let finishes = &c.drawables[d].textures;
            let name = finishes.get(t).cloned().unwrap_or_else(|| "Original".into());
            (format!("  {} finish", c.label), format!("{name} ({}/{})", t + 1, finishes.len().max(1)))
        }
        Row::Random => ("Random skater".into(), String::new()),
        Row::Reset => ("Default skater".into(), String::new()),
    }
}

fn step(value: usize, len: usize, dir: i32) -> usize {
    let len = len.max(1) as i32;
    ((value as i32 + dir).rem_euclid(len)) as usize
}

/// Changes a row by `dir` (left -1, right +1); returns the components to
/// apply to the ped.
pub fn change(row: Row, dir: i32, catalog: &Catalog, outfit: &mut Outfit, seed: u64) -> Vec<u8> {
    match row {
        Row::Part(i) | Row::Finish(i) => {
            let c = &catalog.components[i];
            let (d, t) = outfit.worn.get(&c.component).copied().unwrap_or((c.default, 0));
            let (d, t) = match row {
                Row::Part(_) => (step(d, c.drawables.len(), dir), 0),
                _ => (d, step(t, c.drawables[d.min(c.drawables.len() - 1)].textures.len(), dir)),
            };
            outfit.worn.insert(c.component, (d, t));
            vec![c.component]
        }
        Row::Random => {
            let mut x = seed | 1;
            let mut next = |n: usize| {
                // xorshift
                x ^= x << 13;
                x ^= x >> 7;
                x ^= x << 17;
                (x % n.max(1) as u64) as usize
            };
            for c in &catalog.components {
                let d = next(c.drawables.len());
                let t = next(c.drawables[d].textures.len());
                outfit.worn.insert(c.component, (d, t));
            }
            catalog.components.iter().map(|c| c.component).collect()
        }
        Row::Reset => {
            *outfit = Outfit::default_for(catalog);
            catalog.components.iter().map(|c| c.component).collect()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const CATALOG: &str = r#"{"ped": "skate3_skater", "components": [
        {"component": 0, "label": "Head", "default": 1, "drawables": [{"name": "Face A", "textures": ["Light"]}, {"name": "Face B", "textures": ["Light", "Dark"]}]},
        {"component": 11, "label": "Top", "default": 0, "drawables": [{"name": "Tee", "textures": ["Red", "Blue", "Green"]}, {"name": "Hoodie", "textures": ["Grey"]}]}
    ]}"#;

    #[test]
    fn the_outfit_is_built_and_kept() {
        let catalog = Catalog::from_json(CATALOG).unwrap();
        let mut outfit = Outfit::default_for(&catalog);
        assert_eq!(outfit.worn[&0], (1, 0), "the default skater's head");
        let rows = rows(&catalog);
        assert_eq!(rows.len(), 6);
        assert_eq!(row_text(rows[0], &catalog, &outfit), ("Head".into(), "Face B (2/2)".into()));
        assert_eq!(change(Row::Finish(1), -1, &catalog, &mut outfit, 1), vec![11]);
        assert_eq!(outfit.worn[&11], (0, 2), "wraps round to the last finish");
        assert_eq!(row_text(Row::Finish(1), &catalog, &outfit).1, "Green (3/3)");
        change(Row::Part(1), 1, &catalog, &mut outfit, 1);
        assert_eq!(outfit.worn[&11], (1, 0), "a new part starts at its first finish");
        let back = Outfit::from_text(&outfit.to_text(), &catalog);
        assert_eq!(back, outfit);
        let damaged = Outfit::from_text("11 9 0\n0 0 5\nnonsense\n", &catalog);
        assert_eq!(damaged, Outfit::default_for(&catalog));
        let changed = change(Row::Random, 1, &catalog, &mut outfit, 12345);
        assert_eq!(changed, vec![0, 11]);
        for c in &catalog.components {
            let (d, t) = outfit.worn[&c.component];
            assert!(d < c.drawables.len() && t < c.drawables[d].textures.len());
        }
        change(Row::Reset, 1, &catalog, &mut outfit, 0);
        assert_eq!(outfit, Outfit::default_for(&catalog));
        assert!(Catalog::from_json(r#"{"ped": "x", "components": [{"component": 12, "label": "?", "drawables": [{"name": "a"}]}]}"#).is_err());
    }
}
