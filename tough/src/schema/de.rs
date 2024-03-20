use crate::schema::decoded::{Decoded, Hex};
use crate::schema::error;
use crate::schema::key::Key;
use serde::{de::Error as _, Deserialize, Deserializer};
use snafu::ensure;
use std::collections::HashMap;
use std::fmt;

/// Validates the key ID for each key during deserialization and fails if any don't match.
pub(super) fn deserialize_keys<'de, D>(
    deserializer: D,
) -> Result<HashMap<Decoded<Hex>, Key>, D::Error>
where
    D: Deserializer<'de>,
{
    // An inner function that does actual key ID validation:
    // * fails if a key ID doesn't match its contents
    // * fails if there is a duplicate key ID
    // If this passes we insert the entry.
    fn validate_and_insert_entry(
        keyid: Decoded<Hex>,
        key: Key,
        map: &mut HashMap<Decoded<Hex>, Key>,
    ) -> Result<(), error::Error> {
        let calculated = key.key_id()?;
        let keyid_hex = hex::encode(&keyid);
        ensure!(
            keyid == calculated,
            error::InvalidKeyIdSnafu {
                keyid: &keyid_hex,
                calculated: hex::encode(&calculated),
            }
        );
        ensure!(
            map.insert(keyid, key).is_none(),
            error::DuplicateKeyIdSnafu { keyid: keyid_hex }
        );
        Ok(())
    }

    // The rest of this is fitting the above function into serde and doing error type conversion.
    struct Visitor;

    impl<'de> serde::de::Visitor<'de> for Visitor {
        type Value = HashMap<Decoded<Hex>, Key>;

        fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            formatter.write_str("a map")
        }

        fn visit_map<M>(self, mut access: M) -> Result<Self::Value, M::Error>
        where
            M: serde::de::MapAccess<'de>,
        {
            let mut map = HashMap::new();
            while let Some((keyid, key)) = access.next_entry()? {
                validate_and_insert_entry(keyid, key, &mut map).map_err(M::Error::custom)?;
            }
            Ok(map)
        }
    }

    deserializer.deserialize_map(Visitor)
}

/// Deserializes the `_extra` field on roles, skipping the `_type` tag.
pub(super) fn extra_skip_type<'de, D>(
    deserializer: D,
) -> Result<HashMap<String, serde_json::Value>, D::Error>
where
    D: Deserializer<'de>,
{
    let mut map = HashMap::deserialize(deserializer)?;
    map.remove("_type");
    Ok(map)
}

#[cfg(test)]
mod tests {
    use crate::schema::{Root, Signed};

    #[test]
    fn duplicate_keyid() {
        assert!(serde_json::from_str::<Signed<Root>>(include_str!(
            "../../tests/data/duplicate-keyid/root.json"
        ))
        .is_err());
    }

    /// Ensure that we can deserialize a root.json file that has hex-encoded ECDSA keys. This uses
    /// sigstore's root.json file taken from here:
    /// `<https://sigstore-tuf-root.storage.googleapis.com/2.root.json>`
    #[test]
    fn ecdsa_hex_encoded_keys() {
        assert!(serde_json::from_str::<Signed<Root>>(include_str!(
            "../../tests/data/hex-encoded-ecdsa-sig-keys/root.json"
        ))
        .is_ok());
    }

    /// Ensure that we can deserialize a root.json file that has pem-encoded ECDSA keys. This uses
    /// sigstore's root.json file taken from here:
    /// `<https://github.com/sigstore/sigstore-rs/blob/8a269a3/trust_root/prod/root.json>`
    #[test]
    fn ecdsa_pem_encoded_keys() {
        assert!(serde_json::from_str::<Signed<Root>>(include_str!(
            "../../tests/data/pem-encoded-ecdsa-sig-keys/root.json"
        ))
        .is_ok());
    }
    /// Ensure that we can deserialize a root.json file that has ECDSA keys with new type ecdsa. This uses
    /// sigstore's root.json file taken from here:
    /// `<https://github.com/sigstore/root-signing/blob/d3738d62e92580b5b928d6212c927084ada2bfee/repository/repository/9.root.json>`
    #[test]
    fn ecdsa_new_type_keys() {
        assert!(serde_json::from_str::<Signed<Root>>(include_str!(
            "../../tests/data/ecdsa-new-type-sig-keys/root.json"
        ))
        .is_ok());
    }
}
