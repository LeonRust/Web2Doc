//! robots.txt 合规（plan §6.2 / C9 / M5.1）：默认尊重，`--ignore-robots` 短路。
//!
//! 容错优先：robots.txt 不存在 / 拉取失败 / 解析失败 → 全部允许（不因 robots 问题阻断抓取）。

use texting_robots::Robot;
use url::Url;

/// 抓取使用的 User-Agent（与 robots.txt 的 `User-agent` 段匹配）。
const UA: &str = "web2doc";

/// 站点 robots 策略。
pub struct RobotsPolicy {
    /// `None` = 忽略 / 无 robots / 解析失败 → 全部允许。
    robot: Option<Robot>,
}

impl RobotsPolicy {
    /// 全部允许（`--ignore-robots` 或无可用 robots 时）。
    pub fn allow_all() -> Self {
        Self { robot: None }
    }

    /// 由 robots.txt 文本构建；解析失败 → 全部允许。
    pub fn from_txt(txt: &str) -> Self {
        Self {
            robot: Robot::new(UA, txt.as_bytes()).ok(),
        }
    }

    /// 拉取 `<origin>/robots.txt` 构建策略；`ignore=true` 跳过拉取（全部允许）。
    pub async fn load(client: &reqwest::Client, start: &Url, ignore: bool) -> Self {
        if ignore {
            return Self::allow_all();
        }
        let Ok(robots_url) = start.join("/robots.txt") else {
            return Self::allow_all();
        };
        match client.get(robots_url).send().await {
            Ok(resp) if resp.status().is_success() => match resp.text().await {
                Ok(txt) => Self::from_txt(&txt),
                Err(_) => Self::allow_all(),
            },
            _ => Self::allow_all(),
        }
    }

    /// 该 URL 是否允许抓取。
    pub fn is_allowed(&self, url: &Url) -> bool {
        match &self.robot {
            Some(r) => r.allowed(url.as_str()),
            None => true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn u(s: &str) -> Url {
        Url::parse(s).unwrap()
    }

    #[test]
    fn disallow_blocks_matching_path() {
        let p = RobotsPolicy::from_txt("User-agent: *\nDisallow: /secret/");
        assert!(!p.is_allowed(&u("https://x.com/secret/a")));
        assert!(p.is_allowed(&u("https://x.com/docs/a")));
    }

    #[test]
    fn allow_all_and_empty_are_permissive() {
        assert!(RobotsPolicy::allow_all().is_allowed(&u("https://x.com/secret/a")));
        let empty = RobotsPolicy::from_txt("");
        assert!(empty.is_allowed(&u("https://x.com/anything")));
    }
}
