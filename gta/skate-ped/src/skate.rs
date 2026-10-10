//! The Skate 3 side: the skater's skeleton (from the converted default
//! skater) and the skinned meshes of Create-a-Skater parts, from the GLBs
//! the Skate 3 converter writes on the player's machine.

use std::collections::HashMap;

use glam::{Mat4, Vec2, Vec3};
use serde_json::Value;
use skate_boards::glb::{self, Glb};

/// A joint in Skate space (metres, Y up): where it sits in the bind pose,
/// and its parent's name.
#[derive(Clone, Debug)]
pub struct Joint {
    pub position: Vec3,
    pub parent: Option<String>,
}

/// The skater's skeleton, by joint name.
#[derive(Clone, Debug, Default)]
pub struct Rig {
    pub joints: HashMap<String, Joint>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Vertex {
    pub position: Vec3,
    pub normal: Vec3,
    pub uv: Vec2,
    /// Up to four (joint name, weight), weights summing to 1.
    pub influences: Vec<(String, f32)>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Surface {
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u32>,
}

fn skin_joints(glb: &Glb) -> Result<(Vec<String>, Vec<Mat4>, HashMap<usize, usize>), String> {
    let skin = &glb.json["skins"][0];
    let joints = skin["joints"].as_array().ok_or("the model has no skin")?;
    let inverse = glb.floats(glb::index(&skin["inverseBindMatrices"])?)?;
    if inverse.len() != joints.len() * 16 {
        return Err("bind matrices do not match the joints".into());
    }
    let mut names = Vec::new();
    let mut binds = Vec::new();
    let mut node_of = HashMap::new();
    for (i, joint) in joints.iter().enumerate() {
        let node = glb::index(joint)?;
        node_of.insert(node, i);
        names.push(glb.json["nodes"][node]["name"].as_str().ok_or("a joint has no name")?.to_owned());
        binds.push(Mat4::from_cols_slice(&inverse[i * 16..i * 16 + 16]).inverse());
    }
    Ok((names, binds, node_of))
}

/// Each node's parent node.
fn parents(json: &Value) -> HashMap<usize, usize> {
    let mut out = HashMap::new();
    for (i, node) in json["nodes"].as_array().into_iter().flatten().enumerate() {
        for child in node["children"].as_array().into_iter().flatten() {
            if let Some(c) = child.as_u64() {
                out.insert(c as usize, i);
            }
        }
    }
    out
}

impl Rig {
    /// The skeleton of a skinned GLB.
    pub fn from_glb(bytes: &[u8]) -> Result<Self, String> {
        let glb = Glb::parse(bytes)?;
        let (names, binds, node_of) = skin_joints(&glb)?;
        let parent_nodes = parents(&glb.json);
        let mut rig = Rig::default();
        for (node, &i) in &node_of {
            // The nearest ancestor that is a joint.
            let mut at = parent_nodes.get(node).copied();
            let mut parent = None;
            while let Some(p) = at {
                if let Some(&j) = node_of.get(&p) {
                    parent = Some(names[j].clone());
                    break;
                }
                at = parent_nodes.get(&p).copied();
            }
            rig.joints.insert(names[i].clone(), Joint { position: binds[i].w_axis.truncate(), parent });
        }
        Ok(rig)
    }

    /// Adds the joints of another skinned GLB the rig lacks.
    pub fn merge(&mut self, other: Rig) {
        for (name, joint) in other.joints {
            self.joints.entry(name).or_insert(joint);
        }
    }

    pub fn position(&self, name: &str) -> Option<Vec3> {
        self.joints.get(name).map(|j| j.position)
    }
}

/// The skinned surfaces of a part GLB.
pub fn surfaces(bytes: &[u8]) -> Result<(Vec<Surface>, Rig), String> {
    let rig = Rig::from_glb(bytes)?;
    let glb = Glb::parse(bytes)?;
    let (names, _, _) = skin_joints(&glb)?;
    let mut out = Vec::new();
    for mesh in glb.json["meshes"].as_array().into_iter().flatten() {
        for primitive in mesh["primitives"].as_array().into_iter().flatten() {
            let a = &primitive["attributes"];
            let positions = glb.floats(glb::index(&a["POSITION"])?)?;
            let count = positions.len() / 3;
            let normals = match a.get("NORMAL") {
                Some(n) => glb.floats(glb::index(n)?)?,
                None => vec![0.; count * 3],
            };
            let uvs = match a.get("TEXCOORD_0") {
                Some(t) => glb.floats(glb::index(t)?)?,
                None => vec![0.; count * 2],
            };
            let (Some(j), Some(w)) = (a.get("JOINTS_0"), a.get("WEIGHTS_0")) else { continue };
            let joints = glb.integers(glb::index(j)?)?;
            let weights = glb.floats(glb::index(w)?)?;
            if normals.len() != count * 3 || uvs.len() != count * 2 || joints.len() != count * 4 || weights.len() != count * 4 {
                return Err("vertex streams disagree".into());
            }
            let indices = match primitive.get("indices") {
                Some(i) => glb.integers(glb::index(i)?)?,
                None => (0..count as u32).collect(),
            };
            if indices.len() % 3 != 0 || indices.iter().any(|&i| i as usize >= count) {
                return Err("broken triangles".into());
            }
            let mut vertices = Vec::with_capacity(count);
            for v in 0..count {
                let mut influences: Vec<(String, f32)> = Vec::new();
                for k in 0..4 {
                    let weight = weights[v * 4 + k];
                    let joint = joints[v * 4 + k] as usize;
                    if weight > 0. && joint < names.len() {
                        influences.push((names[joint].clone(), weight));
                    }
                }
                let total: f32 = influences.iter().map(|i| i.1).sum();
                if total <= 0. {
                    influences = vec![(names[0].clone(), 1.)];
                } else {
                    for i in &mut influences {
                        i.1 /= total;
                    }
                }
                vertices.push(Vertex {
                    position: Vec3::from_slice(&positions[v * 3..v * 3 + 3]),
                    normal: Vec3::from_slice(&normals[v * 3..v * 3 + 3]),
                    uv: Vec2::new(uvs[v * 2], uvs[v * 2 + 1]),
                    influences,
                });
            }
            out.push(Surface { vertices, indices });
        }
    }
    if out.is_empty() {
        return Err("the model has no skinned surfaces".into());
    }
    Ok((out, rig))
}

#[cfg(test)]
pub(crate) mod testing {
    use serde_json::json;

    /// A skinned GLB: joints HIPS (at 1 m), HEAD (at 1.7 m), the thighs
    /// and the left foot, Y up, one triangle on HIPS.
    pub fn glb() -> Vec<u8> {
        let mut blob: Vec<u8> = Vec::new();
        let mut views = Vec::new();
        let mut accessors = Vec::new();
        let mut push = |data: Vec<u8>, accessor: serde_json::Value| {
            let offset = blob.len();
            blob.extend_from_slice(&data);
            while blob.len() % 4 != 0 {
                blob.push(0);
            }
            views.push(json!({"buffer": 0, "byteOffset": offset, "byteLength": data.len()}));
            let mut accessor = accessor;
            accessor["bufferView"] = json!(views.len() - 1);
            accessors.push(accessor);
            accessors.len() - 1
        };
        let f = |v: &[f32]| v.iter().flat_map(|x| x.to_le_bytes()).collect::<Vec<u8>>();
        let ibm = |x: f32, y: f32| [1., 0., 0., 0., 0., 1., 0., 0., 0., 0., 1., 0., -x, -y, 0., 1.];
        let mut inv = ibm(0., 1.).to_vec();
        for (x, y) in [(0., 1.7), (0.1, 1.), (-0.1, 1.), (0.1, 0.1)] {
            inv.extend(ibm(x, y));
        }
        let ibm_acc = push(f(&inv), json!({"componentType": 5126, "count": 5, "type": "MAT4"}));
        let pos = push(f(&[0., 1., 0., 0.1, 1., 0., 0., 1.1, 0.]), json!({"componentType": 5126, "count": 3, "type": "VEC3"}));
        let nrm = push(f(&[0., 0., 1., 0., 0., 1., 0., 0., 1.]), json!({"componentType": 5126, "count": 3, "type": "VEC3"}));
        let uv = push(f(&[0., 0., 1., 0., 0., 1.]), json!({"componentType": 5126, "count": 3, "type": "VEC2"}));
        let jnt = push(vec![0; 12], json!({"componentType": 5121, "count": 3, "type": "VEC4"}));
        let wgt = push(f(&[1., 0., 0., 0., 1., 0., 0., 0., 1., 0., 0., 0.]), json!({"componentType": 5126, "count": 3, "type": "VEC4"}));
        let idx = push(vec![0, 0, 1, 0, 2, 0], json!({"componentType": 5123, "count": 3, "type": "SCALAR"}));
        let gltf = json!({
            "asset": {"version": "2.0"},
            "buffers": [{"byteLength": blob.len()}],
            "bufferViews": views,
            "accessors": accessors,
            "nodes": [
                {"name": "HIPS", "children": [1, 2, 3], "translation": [0, 1, 0]},
                {"name": "HEAD", "translation": [0, 0.7, 0]},
                {"name": "LEFTUPLEG", "children": [4], "translation": [0.1, 0, 0]},
                {"name": "RIGHTUPLEG", "translation": [-0.1, 0, 0]},
                {"name": "LEFTFOOT", "translation": [0, -0.9, 0]},
                {"mesh": 0, "skin": 0}
            ],
            "skins": [{"joints": [0, 1, 2, 3, 4], "inverseBindMatrices": ibm_acc}],
            "meshes": [{"primitives": [{"attributes": {"POSITION": pos, "NORMAL": nrm, "TEXCOORD_0": uv, "JOINTS_0": jnt, "WEIGHTS_0": wgt}, "indices": idx}]}]
        });
        let mut json_bytes = serde_json::to_vec(&gltf).unwrap();
        while json_bytes.len() % 4 != 0 {
            json_bytes.push(b' ');
        }
        let mut out = b"glTF".to_vec();
        out.extend(2u32.to_le_bytes());
        out.extend(((12 + 8 + json_bytes.len() + 8 + blob.len()) as u32).to_le_bytes());
        out.extend((json_bytes.len() as u32).to_le_bytes());
        out.extend(b"JSON");
        out.extend(json_bytes);
        out.extend((blob.len() as u32).to_le_bytes());
        out.extend(b"BIN\0");
        out.extend(blob);
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_the_rig_and_the_surfaces() {
        let bytes = testing::glb();
        let rig = Rig::from_glb(&bytes).unwrap();
        assert!((rig.position("HEAD").unwrap().y - 1.7).abs() < 1e-5);
        assert_eq!(rig.joints["HEAD"].parent.as_deref(), Some("HIPS"));
        assert_eq!(rig.joints["HIPS"].parent, None);
        let (surfaces, _) = surfaces(&bytes).unwrap();
        assert_eq!(surfaces[0].indices, [0, 1, 2]);
        assert_eq!(surfaces[0].vertices[0].influences, [("HIPS".to_owned(), 1.)]);
    }
}
