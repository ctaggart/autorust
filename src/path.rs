use crate::uri_util::uri_join;

use super::Result;

use http::Uri;

pub fn path_join(a: &str, b: &str) -> Result<String> {
    let a = a.parse::<Uri>()?;
    let c = uri_join(a, b)?;
    Ok(c.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    // #[test]
    fn test_path_join() -> Result<()> {
        let a = "../azure-rest-api-specs/specification/vmware/resource-manager/Microsoft.AVS/stable/2020-03-20/vmware.json";
        let b = "../../../../../common-types/resource-management/v1/types.json";
        let c = path_join(a, b)?;
        assert_eq!(
            c,
            "../azure-rest-api-specs/common-types/resource-management/v1/types.json"
        );
        Ok(())
    }
}
