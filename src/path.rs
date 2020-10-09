use crate::Result;
use path_abs::PathMut;
use std::path::{Path, PathBuf};

pub fn join<P1: AsRef<Path>, P2: AsRef<Path>>(from_file: bool, a: P1, b: P2) -> Result<PathBuf> {
    let mut c = PathBuf::from(a.as_ref());
    if from_file {
        c.pop_up()?; // to directory
    }
    c.append(b)?;
    Ok(c)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_path_join() -> Result<()> {
        let a = "../azure-rest-api-specs/specification/vmware/resource-manager/Microsoft.AVS/stable/2020-03-20/vmware.json";
        let b = "../../../../../common-types/resource-management/v1/types.json";
        let c = join(true, a, b)?;
        assert_eq!(
            c,
            PathBuf::from("../azure-rest-api-specs/specification/common-types/resource-management/v1/types.json")
        );
        Ok(())
    }
}
