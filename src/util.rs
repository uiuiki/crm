//! # 工具模块
//!
//! 工具模块中包含一些简单的函数。

#![allow(deprecated)]

use std::{
    env,
    ffi::OsStr,
    fmt::Display,
    fs::read_to_string,
    io,
    path::{Path, PathBuf},
    process,
    sync::mpsc,
    thread,
    time::{Duration, SystemTime},
};

use crate::constants::{CARGO, CARGO_CONFIG_PATH, CARGO_HOME, CONFIG, UNC_PREFIX};

pub fn home_dir() -> PathBuf {
    env::home_dir().unwrap()
}

pub fn cargo_home() -> PathBuf {
    match env::var_os(CARGO_HOME) {
        Some(value) => PathBuf::from(value),
        None => home_dir().clone().join(CARGO),
    }
}

pub fn cargo_config_path() -> PathBuf {
    let mut c = cargo_home();
    c.push(CONFIG);

    c
}

pub fn get_cargo_config() -> String {
    match read_to_string(cargo_config_path()) {
        Ok(content) => content,
        Err(_) => "".to_string(),
    }
}

pub fn is_registry_name(name: Option<&String>) -> &str {
    if name.is_none() {
        to_out("请输入正确的镜像名");
        process::exit(-1);
    }

    name.unwrap().as_str()
}

pub fn is_registry_addr(addr: Option<&String>) -> &str {
    if addr.is_none() {
        to_out("请输入正确的镜像地址");
        process::exit(-1);
    }

    addr.unwrap().as_str()
}

pub fn is_registry_dl(dl: Option<&String>) -> &str {
    if dl.is_none() {
        to_out("请输入正确的 dl。\n  每一个镜像源都是一个 Git 存储库，而在该存储库的根目录下有一个 config.json 文件，\n  其中，dl 属性是 config.json 文件中的一个字段。");
        process::exit(-1);
    }

    dl.unwrap().as_str()
}

pub fn append_end_spaces(value: &str, total_len: Option<usize>) -> String {
    let size = if total_len.is_none() {
        15
    } else {
        total_len.unwrap()
    };

    let pad = if value.len() < size {
        " ".repeat(size - value.len())
    } else {
        "".to_string()
    };

    format!("{}{}", value, pad)
}

pub fn request(url: &String) -> Option<u128> {
    let time = SystemTime::now();

    match ureq::get(url).timeout(Duration::from_secs(10)).call() {
        Ok(res) => {
            let status = res.status();

            if status >= 400 {
                return None;
            }

            if status >= 300 {
                return match res.header("location") {
                    Some(v) => request(&v.to_string()),
                    None => None,
                };
            }

            Some(time.elapsed().unwrap().as_millis())
        }
        Err(_) => None,
    }
}

pub fn network_delay(
    values: Vec<(String, Option<String>)>,
    sender_size: Option<usize>,
) -> Vec<(String, Option<u128>)> {
    let (tx, rx) = mpsc::channel();
    let iter = values.iter();
    let len = match sender_size {
        Some(size) => size,
        None => iter.len(),
    };
    let mut ret = vec![];

    for v in iter {
        let t = tx.clone();
        let v = v.clone();

        thread::spawn(move || {
            let date = match &v.1 {
                Some(url) => request(url),
                None => None,
            };

            if let Err(_) = t.send((v.0.to_string(), date)) {
                process::exit(0);
            }
        });
    }

    for _ in 0..len {
        ret.push(rx.recv().unwrap());
    }

    ret.sort_by(|a, b| a.1.cmp(&b.1));

    ret
}

pub fn field_eprint(field_name: &str, field_type: &str) {
    to_out(format!(
        "{} 文件中的 {} 字段不是一个{}，请修改后重试",
        CARGO_CONFIG_PATH, field_name, field_type
    ));
}

pub fn to_out<T: Display>(message: T) {
    println!(" {}", message);
}

pub fn status_prefix(value1: &String, value2: &String) -> String {
    if value1.eq(value2) { "  * " } else { "    " }.to_string()
}

pub fn is_windows() -> bool {
    cfg!(target_os = "windows")
}

pub fn absolute_path<T: AsRef<OsStr>>(dir: &T) -> io::Result<PathBuf> {
    match Path::new(dir).canonicalize() {
        Ok(mut path) => {
            let path_str = path.to_str().unwrap();

            // 如果是 `Windows`，并且当前路径是 `UNC` 路径
            if is_windows() && path_str.starts_with(UNC_PREFIX) {
                let path_slice = &path_str[UNC_PREFIX.len()..];

                // 路径不能超过普通 `Windows` 路径的长度
                if path_slice.len() > 260 {
                    to_out(format!(
                        "当前路径超过了 Windows 普通路径的最大长度: {}",
                        path_str
                    ));
                    process::exit(-1);
                }

                path = PathBuf::from(path_slice);
            }

            Ok(path)
        }
        Err(e) => Err(e),
    }
}

pub fn not_command(command: &str) {
    let r = r#"
  crm best                    评估网络延迟并自动切换到最优的镜像
  crm current                 获取当前所使用的镜像
  crm default                 恢复为默认的镜像
  crm list                    从镜像配置文件中获取镜像列表
  crm publish [cwd]           自动切换镜像源并执行 "cargo publish" 命令
  crm remove <name>           在镜像配置文件中删除镜像
  crm save <name> <addr> <dl> 在镜像配置文件中添加/更新镜像
  crm test [name]             下载测试包以评估网络延迟
  crm use <name>              切换为要使用的镜像
"#;

    to_out(format!("{} 命令无效。参考:\n{}", command, r));
    process::exit(-1);
}
