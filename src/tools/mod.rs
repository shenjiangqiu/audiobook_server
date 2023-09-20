use std::path::{Path, PathBuf};

pub async fn arrange_new_folder(src_dir: impl AsRef<Path>, target_dir: impl AsRef<Path>) -> i32 {
    let files = get_files_in_dir(src_dir);
    let count = files.len();
    // create target dir if not exists
    std::fs::create_dir_all(target_dir.as_ref()).unwrap();
    for (src, target_index) in files.into_iter().zip(1..) {
        // create a hard link from src to target_dir, with new filename target_index+src.ext
        let target = target_dir.as_ref().join(format!(
            "{:04}.{}",
            target_index,
            src.extension().unwrap().to_str().unwrap()
        ));
        tokio::fs::hard_link(src, target).await.unwrap();
    }
    count as i32
}
fn sort_with_number(paths: Vec<PathBuf>) -> Vec<PathBuf> {
    let fist_numer_reg = regex::Regex::new(r"\d+").unwrap();

    let mut out = paths
        .into_iter()
        .filter_map(|file| {
            let fd = fist_numer_reg.find(file.file_name().unwrap().to_str().unwrap());
            match fd {
                Some(m) => {
                    let num = m.as_str().parse::<u32>().unwrap();
                    Some((num, file))
                }
                None => None,
            }
        })
        .collect::<Vec<_>>();
    out.sort_by_key(|(num, _)| *num);
    out.into_iter().map(|(_, file)| file).collect()
}
fn get_files_in_dir(dir: impl AsRef<Path>) -> Vec<PathBuf> {
    let files_and_dirs = std::fs::read_dir(dir).unwrap();
    let entries: Vec<_> = files_and_dirs.into_iter().map(|e| e.unwrap()).collect();
    let mut files = vec![];
    let mut dirs = vec![];
    for e in entries {
        if e.file_type().unwrap().is_dir() {
            dirs.push(e.path());
        } else {
            files.push(e.path());
        }
    }
    let files_sorted = sort_with_number(files);
    let dirs_sorted = sort_with_number(dirs);
    let mut out = vec![];
    out.extend(files_sorted);
    for dir in dirs_sorted {
        out.extend(get_files_in_dir(&dir));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::get_files_in_dir;

    #[test]
    fn test_regex() {
        let text = "11231aabdd22.txt";
        let re = regex::Regex::new(r"\d+").unwrap();
        let cap = re.captures(text).unwrap();
        for c in cap.iter() {
            println!("{:?}", c);
        }
        let fd = re.find(text).unwrap();
        println!("{:?}", fd);

        assert_eq!(cap.get(0).unwrap().as_str(), "11231");
    }

    #[test]
    fn test_get_files() {
        let files = get_files_in_dir("./test_dir");
        for f in files {
            println!("{:?}", f);
        }
    }

    #[tokio::test]
    async fn test_link() {
        let src_dir = "./test_dir";
        let target_dir = "./test_dir2";
        super::arrange_new_folder(src_dir, target_dir).await;
    }
}
