use std::ffi::OsString;
use std::fmt::Formatter;
use std::io::Error as IoError;
use std::path::PathBuf;
use std::{error, fmt, fs};
use tempfile::NamedTempFile;

pub type RenameMapPair = (PathBuf, PathBuf);

#[derive(Clone, Copy)]
pub enum RenameOverwriteMode {
    ChangeFileName,
    Overwrite,
    Error,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct BulkRename {
    pub pairs: Vec<RenameMapPair>,
    undo_pairs: Option<Vec<RenameMapPair>>,
}

impl BulkRename {
    pub fn new(pairs: Vec<RenameMapPair>) -> Self {
        let undo_pairs = Some(Vec::with_capacity(pairs.len()));
        Self { pairs, undo_pairs }
    }

    pub fn fix_target_file_path(target: &PathBuf) -> Result<PathBuf, RenameError> {
        if target.exists() {
            let file_name = target.file_name().ok_or(RenameError::IllegalOperation)?;
            let new_target = (1..)
                .map(|i| {
                    let mut new_file_name = OsString::from("_".repeat(i));
                    new_file_name.push(file_name);
                    target.with_file_name(new_file_name.as_os_str())
                })
                .skip_while(|new_target| new_target.exists())
                .take(1)
                .nth(0)
                .unwrap();
            Ok(new_target)
        } else {
            Ok(target.clone())
        }
    }

    pub fn check_not_found_source_files(&self) -> Result<(), RenameError> {
        let not_found_source_files = self
            .pairs
            .iter()
            .filter(|&(source, _)| !source.exists())
            .map(|v| v.clone())
            .collect::<Vec<_>>();
        if not_found_source_files.len() > 0 {
            return Err(RenameError::SourceFileNotFound(not_found_source_files));
        }

        Ok(())
    }

    pub fn execute(&mut self, over_write_mode: RenameOverwriteMode) -> Result<(), RenameError> {
        if self.undo_pairs.as_ref().map_or(true, |v| v.len() > 0) {
            return Err(RenameError::Executed);
        }
        self.check_not_found_source_files()?;

        // Step 1 Move the all files to temporary name.
        let mut temp_filenames = Vec::with_capacity(self.pairs.len());
        for pair in self.pairs.iter() {
            let target_parent = pair.1.parent().ok_or(RenameError::IllegalOperation)?;
            let target_temp_filename = {
                let temp_file = NamedTempFile::new_in(target_parent).map_err(|error| {
                    RenameError::TargetDirectoryNotWritable(pair.clone(), error)
                })?;
                PathBuf::from(temp_file.path())
            };
            debug_assert!(!target_temp_filename.exists());

            fs::rename(&pair.0, &target_temp_filename)
                .map_err(|error| RenameError::IoError(pair.clone(), error))?;
            if let Some(undo_pairs) = self.undo_pairs.as_mut() {
                undo_pairs.push((target_temp_filename.clone(), pair.0.clone()));
            }
            temp_filenames.push(target_temp_filename);
        }

        // Step 2 Move them to target
        for (i, pair) in self.pairs.iter().enumerate() {
            let target_temp_file = &temp_filenames[i];
            let target_file = match over_write_mode {
                RenameOverwriteMode::ChangeFileName => Self::fix_target_file_path(&pair.1),
                RenameOverwriteMode::Overwrite => {
                    if pair.1.exists() {
                        self.undo_pairs = None; // Mark not undoable
                    }
                    Ok(pair.1.clone())
                }
                RenameOverwriteMode::Error => {
                    if pair.1.exists() {
                        Err(RenameError::TargetFileAlreadyExists(pair.clone()))
                    } else {
                        Ok(pair.1.clone())
                    }
                }
            }?;

            fs::rename(target_temp_file, &target_file)
                .map_err(|error| RenameError::IoError(pair.clone(), error))?;
            if let Some(undo_pairs) = self.undo_pairs.as_mut() {
                undo_pairs[i].0 = target_file;
            }
        }

        Ok(())
    }

    pub fn undo_bulk_rename(&self) -> Option<BulkRename> {
        self.undo_pairs
            .as_ref()
            .map(|undo_pairs| BulkRename::new(undo_pairs.clone()))
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use std::path::Path;

    fn path_buf_join<T: AsRef<Path>>(a: &Path, b: T) -> PathBuf {
        let mut joined = PathBuf::from(a);
        joined.push(b);
        joined
    }

    #[test]
    pub fn test_fix_target_file_path() {
        let temp_dir = tempfile::tempdir().unwrap();
        let a_txt = path_buf_join(temp_dir.path(), "a.txt");

        for i in 0..3 {
            let a_ith_txt = path_buf_join(temp_dir.path(), format!("{}a.txt", "_".repeat(i)));
            fs::write(&a_ith_txt, "a").unwrap();

            assert_eq!(
                BulkRename::fix_target_file_path(&a_txt)
                    .unwrap()
                    .file_name(),
                Some(OsString::from(format!("{}a.txt", "_".repeat(i + 1))).as_os_str())
            );
        }
    }

    #[test]
    pub fn test_execute_when_conflicting() {
        for &mode in &[
            RenameOverwriteMode::ChangeFileName,
            RenameOverwriteMode::Overwrite,
            RenameOverwriteMode::Error,
        ] {
            let temp_dir = tempfile::tempdir().unwrap();

            let file1_path = path_buf_join(temp_dir.path(), "1.txt");
            fs::write(&file1_path, "1").unwrap();
            let file2_path = path_buf_join(temp_dir.path(), "2.txt");
            fs::write(&file2_path, "2").unwrap();
            let rename_pair = (file1_path.clone(), file2_path.clone());

            let mut rename = BulkRename::new(vec![rename_pair]);
            let result = rename.execute(mode);
            let undo_pairs = rename.undo_pairs;

            match mode {
                RenameOverwriteMode::ChangeFileName => {
                    let new_file_path = path_buf_join(temp_dir.path(), "_2.txt");
                    assert_eq!(fs::read_to_string(&new_file_path).unwrap(), "1");
                    assert_eq!(fs::read_to_string(&file2_path).unwrap(), "2");
                    assert_eq!(undo_pairs, Some(vec![(new_file_path, file1_path)]));
                }
                RenameOverwriteMode::Overwrite => {
                    let file2_path = path_buf_join(temp_dir.path(), "2.txt");
                    assert_eq!(fs::read_to_string(&file2_path).unwrap(), "1");
                    assert_eq!(undo_pairs, None);
                }
                RenameOverwriteMode::Error => {
                    assert!(matches!(
                        result,
                        Err(RenameError::TargetFileAlreadyExists(_pair))
                    ));
                    assert!(matches!(&undo_pairs, Some(_vec)));
                    assert_eq!(undo_pairs.as_ref().unwrap()[0].1, file1_path);
                }
            }
        }
    }

    #[test]
    pub fn test_execute_with_across_directories() {
        let temp_dir = tempfile::tempdir().unwrap();
        let temp_dir2 = tempfile::tempdir().unwrap();
        let mut undo_pairs = Vec::new();
        let mut pairs = Vec::new();
        for i in 0..20 {
            let source_path = path_buf_join(temp_dir.path(), format!("{}.txt", i));
            let target_path = path_buf_join(temp_dir2.path(), format!("foobar_{}.txt", i));
            fs::write(&source_path, format!("{}", i)).unwrap();
            undo_pairs.push((target_path.clone(), source_path.clone()));
            pairs.push((source_path, target_path));
        }

        let mut rename = BulkRename::new(pairs);
        rename.execute(RenameOverwriteMode::Error).unwrap();

        for i in 0..20 {
            let target_path = path_buf_join(temp_dir2.path(), format!("foobar_{}.txt", i));
            assert_eq!(fs::read_to_string(target_path).unwrap(), format!("{}", i));
        }

        assert_eq!(rename.undo_pairs, Some(undo_pairs));
    }

    #[test]
    pub fn test_execute_with_renumbering() {
        let temp_dir = tempfile::tempdir().unwrap();
        let mut pairs = Vec::new();
        for i in 0..20 {
            let source_path = path_buf_join(temp_dir.path(), format!("{}.txt", i));
            let target_path = path_buf_join(temp_dir.path(), format!("{}.txt", i + 1));
            fs::write(&source_path, format!("{}", i)).unwrap();
            pairs.push((source_path, target_path));
        }

        let mut rename = BulkRename::new(pairs);
        rename.execute(RenameOverwriteMode::Error).unwrap();

        for i in 0..20 {
            let target_path = path_buf_join(temp_dir.path(), format!("{}.txt", i + 1));
            assert_eq!(fs::read_to_string(target_path).unwrap(), format!("{}", i));
        }

        let mut undo = rename.undo_bulk_rename().unwrap();
        undo.execute(RenameOverwriteMode::Error).unwrap();

        for i in 0..20 {
            let target_path = path_buf_join(temp_dir.path(), format!("{}.txt", i));
            assert_eq!(fs::read_to_string(target_path).unwrap(), format!("{}", i));
        }
    }
}

#[derive(Debug)]
pub enum RenameError {
    Executed,
    SourceFileNotFound(Vec<RenameMapPair>),
    TargetFileAlreadyExists(RenameMapPair),
    TargetDirectoryNotWritable(RenameMapPair, IoError),
    IoError(RenameMapPair, IoError),
    IllegalOperation,
}

impl error::Error for RenameError {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match self {
            RenameError::Executed => None,
            RenameError::SourceFileNotFound(_) => None,
            RenameError::TargetFileAlreadyExists(_) => None,
            RenameError::TargetDirectoryNotWritable(_, error) => Some(error),
            RenameError::IoError(_, error) => Some(error),
            RenameError::IllegalOperation => None,
        }
    }
}

impl fmt::Display for RenameError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            RenameError::Executed => f.write_str("Already Executed"),
            RenameError::SourceFileNotFound(pairs) => f.write_fmt(format_args!(
                "Source Not Found: {}",
                pairs
                    .iter()
                    .map(|(source, _)| source.display().to_string())
                    .collect::<Vec<_>>()
                    .join(", ")
            )),
            RenameError::TargetFileAlreadyExists(pair) => f.write_fmt(format_args!(
                "Target File Already Exists: {}",
                pair.1.display().to_string()
            )),
            RenameError::TargetDirectoryNotWritable(pair, _error) => f.write_fmt(format_args!(
                "Target Directory Not Writable: {}",
                pair.1.display().to_string()
            )),
            RenameError::IoError(pair, _error) => f.write_fmt(format_args!(
                "IO Error: {} -> {}",
                pair.0.display().to_string(),
                pair.1.display().to_string()
            )),
            RenameError::IllegalOperation => f.write_str("Illegal Format"),
        }
    }
}
