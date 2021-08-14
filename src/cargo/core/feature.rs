use cargo_platform::Platform;

use serde::ser;
use serde::Serialize;

use std::cmp::Ordering;
use std::fmt;
use std::rc::Rc;

use crate::util::interning::InternedString;

use super::FeatureValue;

//TODO: Maybe add `.contains()` on vec??

/// Information about a dependency requested by a Cargo manifest.
/// Cheap to copy.
#[derive(PartialEq, Eq, Hash, Clone, Debug)]
pub struct Feature {
    inner: Rc<InnerFeature>,
}

/// The data underlying a `Dependency`.
#[derive(PartialEq, Eq, Hash, Clone, Debug)]
struct InnerFeature {
    name: FeatureValue,

    child_feature_values: Vec<FeatureValue>,

    // This dependency should be used only for this platform.
    // `None` means *all platforms*.
    platform: Option<Platform>,
}

#[derive(Serialize)]
struct SerializedFeature<'a> {
    name: &'a str,
    target: Option<&'a Platform>,
}

impl ser::Serialize for Feature {
    fn serialize<S>(&self, s: S) -> Result<S::Ok, S::Error>
    where
        S: ser::Serializer,
    {
        SerializedFeature {
            name: &*self.name(),
            target: self.platform(),
        }
        .serialize(s)
    }
}

impl Feature {
    /// Attempt to create a `Dependency` from an entry in the manifest.
    pub fn new_feature(
        name: InternedString,
        platform: Option<Platform>,
        children: Vec<InternedString>,
    ) -> Feature {
        Feature {
            inner: Rc::new(InnerFeature {
                name: FeatureValue::new(name),
                platform,
                child_feature_values: children
                    .iter()
                    .map(|child| FeatureValue::new(*child))
                    .collect(),
            }),
        }
    }

    pub fn name(&self) -> InternedString {
        self.inner.name.to_string()
    }

    /// If none, this dependencies must be built for all platforms.
    /// If some, it must only be built for the specified platform.
    pub fn platform(&self) -> Option<&Platform> {
        self.inner.platform.as_ref()
    }


    pub fn children_values(&self) -> &Vec<FeatureValue> {
        &self.inner.child_feature_values
    }

    pub fn is_dep(&self) -> bool {
        self.inner.name.has_dep_prefix()
    }
}

impl PartialOrd for Feature {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Feature {
    fn cmp(&self, other: &Self) -> Ordering {
        self.inner.name.cmp(&other.inner.name)
    }
}

impl fmt::Display for Feature {
    // This trait requires `fmt` with this exact signature.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Write strictly the first element into the supplied output
        // stream: `f`. Returns `fmt::Result` which indicates whether the
        // operation succeeded or failed. Note that `write!` uses syntax which
        // is very similar to `println!`.
        write!(
            f,
            "{}, platform:{} = [{:?}]",
            self.inner.name,
            self.inner
                .platform
                .clone()
                .unwrap_or_else(|| Platform::Name("None".to_string())),
            self.children_values()
        )
    }
}
