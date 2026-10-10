//! The GTA V ped skeleton the skater is rebuilt on, read from the player's
//! CodeWalker export of `mp_m_freemode_01.yft` (`.yft.xml`).

use glam::{Quat, Vec3};

#[derive(Clone, Debug)]
#[allow(dead_code)]
pub struct Bone {
    pub name: String,
    pub tag: u16,
    pub parent: Option<usize>,
    /// Bind pose in the ped's model space (metres, Z up, facing +Y).
    pub position: Vec3,
    pub rotation: Quat,
}

#[derive(Clone, Debug)]
#[allow(dead_code)]
pub struct Skeleton {
    pub bones: Vec<Bone>,
    /// The `Skeleton` element, verbatim, so the add-on ped can carry it.
    pub xml: String,
}

fn child<'a>(node: roxmltree::Node<'a, 'a>, name: &str) -> Option<roxmltree::Node<'a, 'a>> {
    node.children().find(|c| c.is_element() && c.has_tag_name(name))
}

fn attr(node: Option<roxmltree::Node>, name: &str) -> f32 {
    node.and_then(|n| n.attribute(name)).and_then(|v| v.parse().ok()).unwrap_or(0.)
}

fn value(node: roxmltree::Node, name: &str) -> i64 {
    child(node, name).and_then(|n| n.attribute("value")).and_then(|v| v.parse().ok()).unwrap_or(0)
}

impl Skeleton {
    pub fn from_xml(text: &str) -> Result<Self, String> {
        let doc = roxmltree::Document::parse(text).map_err(|e| format!("skeleton XML: {e}"))?;
        let skeleton = doc
            .descendants()
            .find(|n| n.has_tag_name("Skeleton") && child(*n, "Bones").is_some())
            .ok_or("no Skeleton with Bones in this XML (export mp_m_freemode_01.yft as XML in CodeWalker)")?;
        let bones_node = child(skeleton, "Bones").unwrap();
        let mut locals = Vec::new();
        for item in bones_node.children().filter(|c| c.is_element()) {
            let name = child(item, "Name").and_then(|n| n.text()).unwrap_or("").to_owned();
            let t = child(item, "Translation");
            let r = child(item, "Rotation");
            let parent = value(item, "ParentIndex");
            locals.push((
                name,
                value(item, "Tag") as u16,
                (parent >= 0).then_some(parent as usize),
                Vec3::new(attr(t, "x"), attr(t, "y"), attr(t, "z")),
                Quat::from_xyzw(attr(r, "x"), attr(r, "y"), attr(r, "z"), attr(r, "w")).normalize(),
            ));
        }
        if locals.is_empty() {
            return Err("the skeleton has no bones".into());
        }
        let mut bones = Vec::with_capacity(locals.len());
        for (i, (name, tag, parent, _, _)) in locals.iter().enumerate() {
            // global = parentRot * local + parentPos, composed to the root.
            let (mut pos, mut rot) = (locals[i].3, locals[i].4);
            let mut at = *parent;
            let mut guard = 0;
            while let Some(p) = at {
                let (_, _, pp, ppos, prot) = locals.get(p).ok_or("a bone's parent is missing")?;
                pos = *prot * pos + *ppos;
                rot = *prot * rot;
                at = *pp;
                guard += 1;
                if guard > 1000 {
                    return Err("the skeleton's parents loop".into());
                }
            }
            bones.push(Bone { name: name.clone(), tag: *tag, parent: *parent, position: pos, rotation: rot.normalize() });
        }
        let range = skeleton.range();
        Ok(Self { bones, xml: text[range].to_owned() })
    }

    pub fn find(&self, name: &str) -> Option<usize> {
        self.bones.iter().position(|b| b.name.eq_ignore_ascii_case(name))
    }

    pub fn position(&self, name: &str) -> Option<Vec3> {
        self.find(name).map(|i| self.bones[i].position)
    }
}

#[cfg(test)]
pub(crate) const TEST_SKELETON: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<Fragment>
 <Drawable>
  <Skeleton>
   <Bones>
    <Item><Name>SKEL_ROOT</Name><Tag value="0" /><Index value="0" /><ParentIndex value="-1" /><Translation x="0" y="0" z="0" /><Rotation x="0" y="0" z="0" w="1" /></Item>
    <Item><Name>SKEL_Pelvis</Name><Tag value="11816" /><Index value="1" /><ParentIndex value="0" /><Translation x="0" y="0" z="1" /><Rotation x="0" y="0" z="0" w="1" /></Item>
    <Item><Name>SKEL_L_Thigh</Name><Tag value="58271" /><Index value="2" /><ParentIndex value="1" /><Translation x="-0.1" y="0" z="0" /><Rotation x="0" y="0" z="0" w="1" /></Item>
    <Item><Name>SKEL_L_Calf</Name><Tag value="63931" /><Index value="3" /><ParentIndex value="2" /><Translation x="0" y="0" z="-0.45" /><Rotation x="0" y="0" z="0" w="1" /></Item>
    <Item><Name>SKEL_L_Foot</Name><Tag value="14201" /><Index value="4" /><ParentIndex value="3" /><Translation x="0" y="0" z="-0.45" /><Rotation x="0" y="0" z="0" w="1" /></Item>
    <Item><Name>SKEL_R_Thigh</Name><Tag value="51826" /><Index value="5" /><ParentIndex value="1" /><Translation x="0.1" y="0" z="0" /><Rotation x="0" y="0" z="0" w="1" /></Item>
    <Item><Name>SKEL_R_Calf</Name><Tag value="36864" /><Index value="6" /><ParentIndex value="5" /><Translation x="0" y="0" z="-0.45" /><Rotation x="0" y="0" z="0" w="1" /></Item>
    <Item><Name>SKEL_R_Foot</Name><Tag value="52301" /><Index value="7" /><ParentIndex value="6" /><Translation x="0" y="0" z="-0.45" /><Rotation x="0" y="0" z="0" w="1" /></Item>
    <Item><Name>SKEL_Spine0</Name><Tag value="23553" /><Index value="8" /><ParentIndex value="1" /><Translation x="0" y="0" z="0.2" /><Rotation x="0" y="0" z="0" w="1" /></Item>
    <Item><Name>SKEL_Neck_1</Name><Tag value="39317" /><Index value="9" /><ParentIndex value="8" /><Translation x="0" y="0" z="0.4" /><Rotation x="0" y="0" z="0" w="1" /></Item>
    <Item><Name>SKEL_Head</Name><Tag value="31086" /><Index value="10" /><ParentIndex value="9" /><Translation x="0" y="0" z="0.1" /><Rotation x="0" y="0" z="0" w="1" /></Item>
   </Bones>
  </Skeleton>
 </Drawable>
</Fragment>
"#;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bones_compose_to_model_space() {
        let s = Skeleton::from_xml(TEST_SKELETON).unwrap();
        assert_eq!(s.bones.len(), 11);
        assert!((s.position("SKEL_L_Foot").unwrap() - Vec3::new(-0.1, 0., 0.1)).length() < 1e-5);
        assert!((s.position("SKEL_Head").unwrap().z - 1.7).abs() < 1e-5);
        assert_eq!(s.bones[s.find("skel_pelvis").unwrap()].tag, 11816);
        assert!(s.xml.starts_with("<Skeleton>"));
        assert!(Skeleton::from_xml("<Fragment/>").is_err());
    }
}
