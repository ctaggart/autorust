use path_abs::PathMut;
use std::path::PathBuf;

use super::Result;

pub fn path_join(a: &str, b: &str) -> Result<String> {
    let mut c = PathBuf::from(a);
    c.pop_up()?; // to directory
    c.append(b)?;
    Ok(c.as_path().to_str().unwrap().to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_path_join() -> Result<()> {
        let a = "../azure-rest-api-specs/specification/vmware/resource-manager/Microsoft.AVS/stable/2020-03-20/vmware.json";
        let b = "../../../../../common-types/resource-management/v1/types.json";
        let c = path_join(a, b)?;
        assert_eq!(
            c,
            "../azure-rest-api-specs/specification/common-types/resource-management/v1/types.json"
        );
        Ok(())
    }
}
