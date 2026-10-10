//! The CodeWalker XML the add-on ped is imported from: component drawables
//! (`.ydd.xml`), texture dictionaries (`.ytd.xml`), the variation file
//! (`.ymt.xml`), and the DLC's `peds.meta`, `content.xml` and `setup2.xml`.

use std::fmt::Write;

use glam::{Vec2, Vec3, Vec4};

pub const HEADER: &str = "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n";

pub fn escape(text: &str) -> String {
    text.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}

/// A float as CodeWalker reads it: invariant, `.` decimals.
fn f(v: f32) -> String {
    if !v.is_finite() {
        return "0".into();
    }
    let s = format!("{v:.6}");
    let s = s.trim_end_matches('0').trim_end_matches('.');
    if s.is_empty() || s == "-0" { "0".into() } else { s.into() }
}

/// A vertex skinned to the ped skeleton: blend indices are slots in its
/// geometry's bone palette, weights sum to 255.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Vertex {
    pub position: Vec3,
    pub normal: Vec3,
    pub uv: Vec2,
    pub tangent: Vec4,
    pub bones: [u8; 4],
    pub weights: [u8; 4],
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Geometry {
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u16>,
    /// Skeleton bone indices; a vertex's blend index is a slot here.
    pub palette: Vec<usize>,
}

/// Weights (summing to 1) as bytes summing to 255.
pub fn weight_bytes(weights: &[f32]) -> [u8; 4] {
    let mut out = [0u8; 4];
    for (o, w) in out.iter_mut().zip(weights) {
        *o = (w.clamp(0., 1.) * 255.).round() as u8;
    }
    let sum: i32 = out.iter().map(|&b| i32::from(b)).sum();
    if sum != 255 && !weights.is_empty() {
        let fix = (i32::from(out[0]) + 255 - sum).clamp(0, 255);
        out[0] = fix as u8;
    }
    out
}

/// Per-vertex tangents from the UVs (w is the bitangent's handedness).
pub fn tangents(positions: &[Vec3], normals: &[Vec3], uvs: &[Vec2], indices: &[u32]) -> Vec<Vec4> {
    let mut tan = vec![Vec3::ZERO; positions.len()];
    let mut bit = vec![Vec3::ZERO; positions.len()];
    for t in indices.chunks_exact(3) {
        let [a, b, c] = [t[0] as usize, t[1] as usize, t[2] as usize];
        let (e1, e2) = (positions[b] - positions[a], positions[c] - positions[a]);
        let (d1, d2) = (uvs[b] - uvs[a], uvs[c] - uvs[a]);
        let det = d1.x * d2.y - d2.x * d1.y;
        if det.abs() < 1e-12 {
            continue;
        }
        let r = 1. / det;
        let sdir = (e1 * d2.y - e2 * d1.y) * r;
        let tdir = (e2 * d1.x - e1 * d2.x) * r;
        for i in [a, b, c] {
            tan[i] += sdir;
            bit[i] += tdir;
        }
    }
    (0..positions.len())
        .map(|i| {
            let n = normals[i];
            let t = (tan[i] - n * n.dot(tan[i])).try_normalize().unwrap_or_else(|| n.any_orthonormal_vector());
            let w = if n.cross(t).dot(bit[i]) < 0. { -1. } else { 1. };
            t.extend(w)
        })
        .collect()
}

pub struct Shader<'a> {
    pub name: &'a str,
    pub diffuse: &'a str,
    pub normal: &'a str,
    pub specular: &'a str,
}

/// A texture as a dictionary item, its DDS file beside the XML.
pub struct TextureItem<'a> {
    pub name: &'a str,
    pub usage: &'a str,
    pub width: u32,
    pub height: u32,
    pub levels: u32,
    pub format: &'a str,
}

fn texture_items(out: &mut String, indent: &str, items: &[TextureItem]) {
    for t in items {
        let _ = write!(
            out,
            "{indent}<Item>\n{indent} <Name>{n}</Name>\n{indent} <Unk32 value=\"128\" />\n{indent} <Usage>{u}</Usage>\n{indent} <UsageFlags>UNK24</UsageFlags>\n{indent} <ExtraFlags value=\"0\" />\n{indent} <Width value=\"{w}\" />\n{indent} <Height value=\"{h}\" />\n{indent} <MipLevels value=\"{l}\" />\n{indent} <Format>{fmt}</Format>\n{indent} <FileName>{n}.dds</FileName>\n{indent}</Item>\n",
            n = escape(t.name),
            u = t.usage,
            w = t.width,
            h = t.height,
            l = t.levels,
            fmt = t.format,
        );
    }
}

/// A texture dictionary (`.ytd.xml`).
pub fn ytd(items: &[TextureItem]) -> String {
    let mut out = format!("{HEADER}<TextureDictionary>\n");
    texture_items(&mut out, " ", items);
    out.push_str("</TextureDictionary>\n");
    out
}

/// A ped component drawable (`.ydd.xml`): one drawable, skinned to the
/// ped's skeleton (it has none of its own), its normal and specular maps
/// embedded.
pub fn ydd(name: &str, shader: &Shader, embedded: &[TextureItem], geometries: &[Geometry]) -> String {
    let (mut lo, mut hi) = (Vec3::splat(f32::MAX), Vec3::splat(f32::MIN));
    for v in geometries.iter().flat_map(|g| &g.vertices) {
        lo = lo.min(v.position);
        hi = hi.max(v.position);
    }
    if lo.x > hi.x {
        (lo, hi) = (Vec3::ZERO, Vec3::ZERO);
    }
    let centre = (lo + hi) * 0.5;
    let radius = geometries.iter().flat_map(|g| &g.vertices).map(|v| v.position.distance(centre)).fold(0f32, f32::max);
    let mut out = format!("{HEADER}<DrawableDictionary>\n <Item>\n  <Name>{}</Name>\n", escape(name));
    let _ = write!(
        out,
        "  <BoundingSphereCenter x=\"{}\" y=\"{}\" z=\"{}\" />\n  <BoundingSphereRadius value=\"{}\" />\n  <BoundingBoxMin x=\"{}\" y=\"{}\" z=\"{}\" />\n  <BoundingBoxMax x=\"{}\" y=\"{}\" z=\"{}\" />\n",
        f(centre.x), f(centre.y), f(centre.z), f(radius), f(lo.x), f(lo.y), f(lo.z), f(hi.x), f(hi.y), f(hi.z)
    );
    for lod in ["High", "Med", "Low", "Vlow"] {
        let _ = writeln!(out, "  <LodDist{lod} value=\"9998\" />");
    }
    out.push_str("  <FlagsHigh value=\"1\" />\n  <FlagsMed value=\"0\" />\n  <FlagsLow value=\"0\" />\n  <FlagsVlow value=\"0\" />\n");
    out.push_str("  <ShaderGroup>\n   <TextureDictionary>\n");
    texture_items(&mut out, "    ", embedded);
    let _ = write!(
        out,
        "   </TextureDictionary>\n   <Shaders>\n    <Item>\n     <Name>{s}</Name>\n     <FileName>{s}.sps</FileName>\n     <RenderBucket value=\"{bucket}\" />\n     <Parameters>\n",
        s = shader.name,
        bucket = if shader.name.contains("alpha") { 1 } else { 0 },
    );
    for (param, texture) in [("DiffuseSampler", shader.diffuse), ("BumpSampler", shader.normal), ("SpecSampler", shader.specular)] {
        let _ = write!(out, "      <Item name=\"{param}\" type=\"Texture\">\n       <Name>{}</Name>\n      </Item>\n", escape(texture));
    }
    for (param, x) in [("specularFresnel", 0.9), ("specularFalloffMult", 20.), ("specularIntensityMult", 0.2), ("bumpiness", 1.)] {
        let _ = writeln!(out, "      <Item name=\"{param}\" type=\"Vector\" x=\"{}\" y=\"0\" z=\"0\" w=\"0\" />", f(x));
    }
    out.push_str("     </Parameters>\n    </Item>\n   </Shaders>\n  </ShaderGroup>\n  <DrawableModelsHigh>\n   <Item>\n    <RenderMask value=\"255\" />\n    <Flags value=\"1\" />\n    <HasSkin value=\"1\" />\n    <BoneIndex value=\"0\" />\n    <Unknown1 value=\"0\" />\n    <Geometries>\n");
    for g in geometries {
        let (mut glo, mut ghi) = (Vec3::splat(f32::MAX), Vec3::splat(f32::MIN));
        for v in &g.vertices {
            glo = glo.min(v.position);
            ghi = ghi.max(v.position);
        }
        let palette: Vec<String> = g.palette.iter().map(|b| b.to_string()).collect();
        let _ = write!(
            out,
            "     <Item>\n      <ShaderIndex value=\"0\" />\n      <BoundingBoxMin x=\"{}\" y=\"{}\" z=\"{}\" w=\"0\" />\n      <BoundingBoxMax x=\"{}\" y=\"{}\" z=\"{}\" w=\"0\" />\n      <BoneIDs>{}</BoneIDs>\n      <VertexBuffer>\n       <Flags value=\"0\" />\n       <Layout type=\"GTAV1\">\n        <Position />\n        <BlendWeights />\n        <BlendIndices />\n        <Normal />\n        <Colour0 />\n        <Colour1 />\n        <TexCoord0 />\n        <Tangent />\n       </Layout>\n       <Data>\n",
            f(glo.x), f(glo.y), f(glo.z), f(ghi.x), f(ghi.y), f(ghi.z),
            palette.join(", ")
        );
        for v in &g.vertices {
            let _ = writeln!(
                out,
                "        {} {} {}   {} {} {} {}   {} {} {} {}   {} {} {}   255 255 255 255   255 255 255 255   {} {}   {} {} {} {}",
                f(v.position.x), f(v.position.y), f(v.position.z),
                v.weights[0], v.weights[1], v.weights[2], v.weights[3],
                v.bones[0], v.bones[1], v.bones[2], v.bones[3],
                f(v.normal.x), f(v.normal.y), f(v.normal.z),
                f(v.uv.x), f(v.uv.y),
                f(v.tangent.x), f(v.tangent.y), f(v.tangent.z), f(v.tangent.w),
            );
        }
        out.push_str("       </Data>\n      </VertexBuffer>\n      <IndexBuffer>\n       <Data>");
        for (i, chunk) in g.indices.chunks(24).enumerate() {
            if i > 0 {
                out.push_str("\n        ");
            }
            let line: Vec<String> = chunk.iter().map(|x| x.to_string()).collect();
            out.push_str(&line.join(" "));
        }
        out.push_str("</Data>\n      </IndexBuffer>\n     </Item>\n");
    }
    out.push_str("    </Geometries>\n   </Item>\n  </DrawableModelsHigh>\n </Item>\n</DrawableDictionary>\n");
    out
}

/// One component's drawables: how many textures each has.
pub struct Component {
    pub slot: u8,
    pub textures: Vec<u8>,
}

/// The ped's variation file (`CPedVariationInfo`, `.ymt.xml`).
pub fn ymt(name: &str, components: &[Component]) -> String {
    let mut avail = [255u8; 12];
    let mut sorted: Vec<&Component> = components.iter().filter(|c| !c.textures.is_empty()).collect();
    sorted.sort_by_key(|c| c.slot);
    for (i, c) in sorted.iter().enumerate() {
        avail[usize::from(c.slot)] = i as u8;
    }
    let avail: Vec<String> = avail.iter().map(|a| a.to_string()).collect();
    let mut out = format!(
        "{HEADER}<CPedVariationInfo name=\"{}\">\n <bHasTexVariations value=\"true\" />\n <bHasDrawblVariations value=\"true\" />\n <bHasLowLODs value=\"false\" />\n <bIsSuperLOD value=\"false\" />\n <availComp>{}</availComp>\n <aComponentData3 itemType=\"CPVComponentData\">\n",
        escape(name),
        avail.join(" ")
    );
    for c in &sorted {
        let total: u32 = c.textures.iter().map(|&t| u32::from(t)).sum();
        let _ = write!(out, "  <Item>\n   <numAvailTex value=\"{}\" />\n   <aDrawblData3 itemType=\"CPVDrawblData\">\n", total.min(255));
        for &count in &c.textures {
            out.push_str("    <Item>\n     <propMask value=\"1\" />\n     <numAlternatives value=\"0\" />\n     <aTexData itemType=\"CPVTextureData\">\n");
            for _ in 0..count {
                out.push_str("      <Item>\n       <texId value=\"0\" />\n       <distribution value=\"255\" />\n      </Item>\n");
            }
            out.push_str("     </aTexData>\n     <clothData>\n      <ownsCloth value=\"false\" />\n     </clothData>\n    </Item>\n");
        }
        out.push_str("   </aDrawblData3>\n  </Item>\n");
    }
    out.push_str(" </aComponentData3>\n <aSelectionSets itemType=\"CPedSelectionSet\" />\n <compInfos itemType=\"CComponentInfo\">\n");
    for c in &sorted {
        for d in 0..c.textures.len() {
            let _ = write!(
                out,
                "  <Item>\n   <pedXml_audioID>none</pedXml_audioID>\n   <pedXml_audioID2 />\n   <pedXml_expressionMods>0 0 0 0 0</pedXml_expressionMods>\n   <flags value=\"0\" />\n   <inclusions>0</inclusions>\n   <exclusions>0</exclusions>\n   <pedXml_vfxComps>PV_COMP_HEAD</pedXml_vfxComps>\n   <pedXml_flags value=\"0\" />\n   <pedXml_compIdx value=\"{}\" />\n   <pedXml_drawblIdx value=\"{d}\" />\n  </Item>\n",
                c.slot
            );
        }
    }
    out.push_str(" </compInfos>\n <propInfo>\n  <numAvailProps value=\"0\" />\n  <aPropMetaData itemType=\"CPedPropMetaData\" />\n  <aAnchors itemType=\"CAnchorProps\" />\n </propInfo>\n <dlcName />\n</CPedVariationInfo>\n");
    out
}

/// The DLC's `peds.meta`: the ped, set up like Los Santos' own skaters.
pub fn peds_meta(name: &str) -> String {
    let fields: [(&str, &str); 52] = [
        ("Name", name), ("PropsName", "null"), ("ClipDictionaryName", "move_m@generic"), ("BlendShapeFileName", "null"),
        ("ExpressionSetName", "expr_set_ambient_male"), ("ExpressionDictionaryName", "null"), ("ExpressionName", "null"),
        ("Pedtype", "CIVMALE"), ("MovementClipSet", "move_m@generic"), ("StrafeClipSet", "move_ped_strafing"),
        ("MovementToStrafeClipSet", "move_ped_to_strafe"), ("InjuredStrafeClipSet", "move_strafe_injured"),
        ("FullBodyDamageClipSet", "dam_ko"), ("AdditiveDamageClipSet", "dam_ad"), ("DefaultGestureClipSet", "ANIM_GROUP_GESTURE_M_GENERIC"),
        ("FacialClipsetGroupName", "facial_clipset_group_gen_male"), ("DefaultVisemeClipSet", "ANIM_GROUP_VISEMES_M_LO"),
        ("SidestepClipSet", "CLIP_SET_ID_INVALID"), ("PoseMatcherName", "Male"), ("PoseMatcherProneName", "Male_prone"),
        ("GetupSetHash", "NMBS_SLOW_GETUPS"), ("CreatureMetadataName", "null"), ("DecisionMakerName", "DEFAULT"),
        ("MotionTaskDataSetName", "STANDARD_PED"), ("DefaultTaskDataSetName", "STANDARD_PED"), ("PedCapsuleName", "STANDARD_MALE"),
        ("PedLayoutName", ""), ("PedComponentSetName", ""), ("PedComponentClothName", ""), ("PedIKSettingsName", ""), ("TaskDataName", ""),
        // Its parts are a folder beside the .yft and .ymt: a streamed ped.
        ("IsStreamedGfx", "=true"), ("AmbulanceShouldRespondTo", "=true"), ("CanRideBikeWithNoHelmet", "=false"),
        ("CanSpawnInCar", "=true"), ("IsHeadBlendPed", "=false"), ("bOnlyBulkyItemVariations", "=false"),
        ("RelationshipGroup", "CIVMALE"), ("NavCapabilitiesName", "STANDARD_PED"), ("PerceptionInfo", "DEFAULT_PERCEPTION"),
        ("DefaultBrawlingStyle", "BS_AI"), ("DefaultUnarmedWeapon", "WEAPON_UNARMED"), ("Personality", "Streamed_Male"),
        ("CombatInfo", "DEFAULT"), ("VfxInfoName", "VFXPEDINFO_HUMAN_GENERIC"), ("AmbientClipsForFlee", "FLEE"),
        ("Radio1", "RADIO_GENRE_PUNK"), ("Radio2", "RADIO_GENRE_MODERN_ROCK"), ("FUpOffset", "=0.000000"), ("RUpOffset", "=0.000000"),
        ("FFrontOffset", "=0.000000"), ("RFrontOffset", "=0.147000"),
    ];
    let rest: [(&str, &str); 18] = [
        ("MinActivationImpulse", "=20.000000"), ("Stubble", "=0.000000"), ("HDDist", "=3.000000"), ("TargetingThreatModifier", "=1.000000"),
        ("KilledPerceptionRangeModifer", "=-1.000000"), ("Sexiness", ""), ("Age", "=0"), ("MaxPassengersInCar", "=0"),
        ("ExternallyDrivenDOFs", ""), ("PedVoiceGroup", ""), ("AnimalAudioObject", ""), ("AbilityType", "SAT_NONE"),
        ("ThermalBehaviour", "TB_WARM"), ("SuperlodType", "SLOD_HUMAN"), ("ScenarioPopStreamingSlot", "SCENARIO_POP_STREAMING_NORMAL"),
        ("DefaultSpawningPreference", "DSP_NORMAL"), ("DefaultRemoveRangeMultiplier", "=1.000000"), ("AllowCloseSpawning", "=false"),
    ];
    // As Rockstar's DLC peds.meta files: the base game's resident texture
    // dictionary and anims are not declared again.
    let mut out = format!("{HEADER}<CPedModelInfo__InitDataList>\n  <InitDatas>\n    <Item>\n");
    for (key, value) in fields.iter().chain(rest.iter()) {
        if let Some(v) = value.strip_prefix('=') {
            let _ = writeln!(out, "      <{key} value=\"{v}\" />");
        } else if value.is_empty() {
            let _ = writeln!(out, "      <{key} />");
        } else {
            let _ = writeln!(out, "      <{key}>{}</{key}>", escape(value));
        }
    }
    out.push_str("    </Item>\n  </InitDatas>\n  <txdRelationships />\n  <multiTxdRelationships />\n</CPedModelInfo__InitDataList>\n");
    out
}

/// The DLC's `content.xml`: the peds.meta and the ped's archive.
pub fn content(device: &str, archive: &str) -> String {
    let files = [
        // GTA's type for a peds.meta; an unknown type is taken for an archive,
        // and the pack breaks the game's loading.
        (format!("{device}:/common/data/peds.meta"), "PED_METADATA_FILE", false),
        (format!("{device}:/%PLATFORM%/models/cdimages/{archive}.rpf"), "RPF_FILE", true),
    ];
    let mut out = format!("{HEADER}<CDataFileMgr__ContentsOfDataFileXml>\n  <disabledFiles />\n  <includedXmlFiles />\n  <includedDataFiles />\n  <dataFiles>\n");
    for (file, kind, persistent) in &files {
        let _ = write!(
            out,
            "    <Item>\n      <filename>{file}</filename>\n      <fileType>{kind}</fileType>\n      <overlay value=\"false\" />\n      <disabled value=\"true\" />\n      <persistent value=\"{persistent}\" />\n    </Item>\n"
        );
    }
    let _ = write!(
        out,
        "  </dataFiles>\n  <contentChangeSets>\n    <Item>\n      <changeSetName>{set}</changeSetName>\n      <filesToDisable />\n      <filesToEnable>\n",
        set = device.to_ascii_uppercase() + "_AUTOGEN"
    );
    for (file, ..) in &files {
        let _ = writeln!(out, "        <Item>{file}</Item>");
    }
    out.push_str("      </filesToEnable>\n    </Item>\n  </contentChangeSets>\n  <patchFiles />\n</CDataFileMgr__ContentsOfDataFileXml>\n");
    out
}

/// The DLC's `setup2.xml`.
pub fn setup(device: &str, pack: &str) -> String {
    format!(
        "{HEADER}<SSetupData>\n  <deviceName>{device}</deviceName>\n  <datFile>content.xml</datFile>\n  <timeStamp>10/07/2026 12:00:00</timeStamp>\n  <nameHash>{pack}</nameHash>\n  <contentChangeSets />\n  <contentChangeSetGroups>\n    <Item>\n      <NameHash>GROUP_STARTUP</NameHash>\n      <ContentChangeSets>\n        <Item>{set}</Item>\n      </ContentChangeSets>\n    </Item>\n  </contentChangeSetGroups>\n  <startupScript />\n  <scriptCallstackSize value=\"0\" />\n  <type>EXTRACONTENT_COMPAT_PACK</type>\n  <order value=\"30\" />\n  <minorOrder value=\"0\" />\n  <isLevelPack value=\"false\" />\n  <dependencyPackHash />\n  <requiredVersion />\n  <subPackCount value=\"0\" />\n</SSetupData>\n",
        set = device.to_ascii_uppercase() + "_AUTOGEN"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn writes_what_codewalker_reads() {
        assert_eq!(f(1.5), "1.5");
        assert_eq!(f(-0.0), "0");
        assert_eq!(f(9998.), "9998");
        assert_eq!(weight_bytes(&[0.5, 0.5]).iter().map(|&b| u32::from(b)).sum::<u32>(), 255);
        let w = weight_bytes(&[0.333, 0.333, 0.334]);
        assert_eq!(w.iter().map(|&b| u32::from(b)).sum::<u32>(), 255);
        let v = Vertex { position: Vec3::new(0., 0., 1.), normal: Vec3::Z, uv: Vec2::new(0.5, 0.25), tangent: Vec4::new(1., 0., 0., 1.), bones: [0; 4], weights: [255, 0, 0, 0] };
        let g = Geometry { vertices: vec![v; 3], indices: vec![0, 1, 2], palette: vec![11] };
        let shader = Shader { name: "ped", diffuse: "jbib_diff_000_a_uni", normal: "jbib_normal_000", specular: "jbib_spec_000" };
        let embedded = [TextureItem { name: "jbib_normal_000", usage: "NORMAL", width: 8, height: 8, levels: 2, format: "D3DFMT_DXT1" }];
        let text = ydd("jbib_000_u", &shader, &embedded, &[g]);
        let doc = roxmltree::Document::parse(&text).unwrap();
        let data = doc.descendants().find(|n| n.has_tag_name("Data")).unwrap().text().unwrap();
        let row: Vec<&str> = data.lines().map(str::trim).find(|l| !l.is_empty()).unwrap().split_whitespace().collect();
        assert_eq!(row.len(), 3 + 4 + 4 + 3 + 4 + 4 + 2 + 4, "one value per PBBNCCTX component");
        assert!(text.contains("<BoneIDs>11</BoneIDs>"));
        assert!(text.contains("<FileName>jbib_normal_000.dds</FileName>"));
        let ymt = ymt("skate3_skater", &[Component { slot: 11, textures: vec![2, 1] }, Component { slot: 0, textures: vec![1] }]);
        assert!(ymt.contains("<availComp>0 255 255 255 255 255 255 255 255 255 255 1</availComp>"));
        roxmltree::Document::parse(&ymt).unwrap();
        roxmltree::Document::parse(&peds_meta("skate3_skater")).unwrap();
        roxmltree::Document::parse(&content("dlc_skate3", "skate3")).unwrap();
        assert!(content("dlc_skate3", "skate3").contains("<fileType>PED_METADATA_FILE</fileType>"));
        roxmltree::Document::parse(&setup("dlc_skate3", "skate3")).unwrap();
        assert!(peds_meta("x").contains("<IsHeadBlendPed value=\"false\" />"));
        assert!(peds_meta("x").contains("<IsStreamedGfx value=\"true\" />"), "parts in a folder");
    }

    #[test]
    fn tangents_follow_the_uvs() {
        let p = [Vec3::ZERO, Vec3::X, Vec3::Y];
        let n = [Vec3::Z; 3];
        let uv = [Vec2::ZERO, Vec2::X, Vec2::Y];
        let t = tangents(&p, &n, &uv, &[0, 1, 2]);
        assert!((t[0] - Vec4::new(1., 0., 0., 1.)).length() < 1e-5);
    }
}
