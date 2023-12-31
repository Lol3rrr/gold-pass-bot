use serde::{Deserialize, Serialize};

mod custom_serde {
    use serde::de::Visitor;

    pub struct TagVisitor;

    impl<'de> Visitor<'de> for TagVisitor {
        type Value = String;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("Expected a String starting with #...")
        }

        fn visit_string<E>(self, v: String) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            if !v.starts_with("#") {
                return Err(serde::de::Error::invalid_value(
                    serde::de::Unexpected::Str(v.as_str()),
                    &"#",
                ));
            }

            Ok(v)
        }

        fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            if !v.starts_with("#") {
                return Err(serde::de::Error::invalid_value(
                    serde::de::Unexpected::Str(v),
                    &"#",
                ));
            }

            Ok(v.to_owned())
        }
    }
}

#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub struct ClanTag(pub String);

impl<'de> Deserialize<'de> for ClanTag {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let inner = deserializer.deserialize_string(custom_serde::TagVisitor {})?;
        Ok(Self(inner))
    }
}

impl Serialize for ClanTag {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.0.serialize(serializer)
    }
}

#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub struct WarTag(pub String);

impl<'de> Deserialize<'de> for WarTag {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let inner = deserializer.deserialize_string(custom_serde::TagVisitor {})?;
        Ok(Self(inner))
    }
}

impl Serialize for WarTag {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.0.serialize(serializer)
    }
}

#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub struct PlayerTag(pub String);

impl<'de> Deserialize<'de> for PlayerTag {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let inner = deserializer.deserialize_string(custom_serde::TagVisitor {})?;
        Ok(Self(inner))
    }
}

impl Serialize for PlayerTag {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.0.serialize(serializer)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deserialize_clantag() {
        #[derive(Debug, Deserialize)]
        struct Test {
            inner: ClanTag,
        }

        assert_eq!(
            ClanTag("#Testing".to_string()),
            serde_json::from_str::<Test>("{ \"inner\": \"#Testing\" }")
                .unwrap()
                .inner,
        );
    }
}
