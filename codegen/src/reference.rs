use crate::Result;

// https://github.com/OAI/OpenAPI-Specification/blob/master/versions/2.0.md#referenceObject
// https://swagger.io/specification/#relative-references-in-urls

#[derive(Clone, Debug, PartialEq)]
pub struct Reference {
    pub file: Option<String>,
    pub path: Vec<String>,
    pub name: Option<String>,
}

impl Reference {
    pub fn parse(s: &str) -> Result<Reference> {
        match s.find("#/") {
            None => Ok(Reference {
                file: Some(s.to_owned()),
                path: vec![],
                name: None,
            }),
            Some(i) => {
                let (file, s) = if i == 0 {
                    (None, s[i + 2..].to_string())
                } else {
                    (Some(s[0..i].to_string()), s[i + 2..].to_string())
                };
                let path = s.split('/').collect::<Vec<_>>();
                let mut path: Vec<String> = path.into_iter().map(str::to_owned).collect();
                let name = path.pop();
                Ok(Reference { file, path, name })
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn can_parse_commontypes() -> Result<()> {
        assert_eq!(
            Reference::parse("../../../../../common-types/resource-management/v1/types.json#/parameters/SubscriptionIdParameter")?,
            Reference {
                file: Some("../../../../../common-types/resource-management/v1/types.json".to_owned()),
                path: vec!["parameters".to_owned()],
                name: Some("SubscriptionIdParameter".to_owned()),
            }
        );
        Ok(())
    }

    #[test]
    fn can_parse_clouderror() -> Result<()> {
        assert_eq!(
            Reference::parse("#/definitions/CloudError")?,
            Reference {
                file: None,
                path: vec!["definitions".to_owned()],
                name: Some("CloudError".to_owned()),
            }
        );
        Ok(())
    }

    #[test]
    fn can_parse_example() -> Result<()> {
        assert_eq!(
            Reference::parse("./examples/Authorizations_CreateOrUpdate.json")?,
            Reference {
                file: Some("./examples/Authorizations_CreateOrUpdate.json".to_owned()),
                path: vec![],
                name: None,
            }
        );
        Ok(())
    }
}
