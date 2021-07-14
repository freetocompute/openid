use chrono::{DateTime, Duration, Utc};
use serde::{de::Visitor, ser::Serializer, Deserialize, Deserializer, Serialize};
use std::fmt;
extern crate chrono;
use chrono::prelude::*;

/// The bearer token type.
///
/// See [RFC 6750](http://tools.ietf.org/html/rfc6750).
#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, Eq)]
pub struct Bearer {
    pub access_token: String,
    pub scope: Option<String>,
    pub refresh_token: Option<String>,
    #[serde(
        default,
        rename = "expires_in",
        deserialize_with = "expire_in_to_instant",
        serialize_with = "serialize_expire_in"
    )]
    pub expires: Option<DateTime<Utc>>,
    pub id_token: Option<String>,
}

fn expire_in_to_instant<'de, D>(deserializer: D) -> Result<Option<DateTime<Utc>>, D::Error>
where
    D: Deserializer<'de>,
{
    struct ExpireInVisitor;

    impl<'de> Visitor<'de> for ExpireInVisitor {
        type Value = Option<DateTime<Utc>>;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("an integer containing seconds")
        }

        fn visit_none<E>(self) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            Ok(None)
        }

        fn visit_some<D>(self, d: D) -> Result<Option<DateTime<Utc>>, D::Error>
        where
            D: Deserializer<'de>,
        {
            let expire_in: i64 = serde::de::Deserialize::deserialize(d)?;
            if expire_in > 31536000 {
                let naive_datetime = NaiveDateTime::from_timestamp(expire_in, 0);
                let datetime_again: DateTime<Utc> = DateTime::from_utc(naive_datetime, Utc);

                let expr = datetime_again;
                Ok(Some(expr))
            } else {
                let expr = Utc::now() + Duration::seconds(expire_in as i64);
                Ok(Some(expr))
            }
        }
    }

    deserializer.deserialize_option(ExpireInVisitor)
}

fn serialize_expire_in<S: Serializer>(
    dt: &Option<DateTime<Utc>>,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    match dt {
        Some(dt) => serializer.serialize_some(&dt.timestamp()),
        None => serializer.serialize_none(),
    }
}

impl Bearer {
    pub fn expired(&self) -> bool {
        if let Some(expires) = self.expires {
            expires < Utc::now()
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_response_refresh() {
        let json = r#"
            {
                "token_type":"Bearer",
                "access_token":"aaaaaaaa",
                "expires_in":3600,
                "refresh_token":"bbbbbbbb"
            }
        "#;
        let bearer: Bearer = serde_json::from_str(json).unwrap();
        assert_eq!("aaaaaaaa", bearer.access_token);
        assert_eq!(None, bearer.scope);
        assert_eq!(Some("bbbbbbbb".into()), bearer.refresh_token);
        let expires = bearer.expires.unwrap();
        assert!(expires > (Utc::now() + Duration::seconds(3599)));
        assert!(expires <= (Utc::now() + Duration::seconds(3600)));
    }

    #[test]
    fn from_response_static() {
        let json = r#"
            {
                "token_type":"Bearer",
                "access_token":"aaaaaaaa"
            }
        "#;
        let bearer: Bearer = serde_json::from_str(json).unwrap();
        assert_eq!("aaaaaaaa", bearer.access_token);
        assert_eq!(None, bearer.scope);
        assert_eq!(None, bearer.refresh_token);
        assert_eq!(None, bearer.expires);
    }
}
