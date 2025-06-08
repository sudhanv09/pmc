use regex::Regex;

/// Extracts the season number from a file or directory name.
/// Returns 0 if not found.
pub fn guess_season(item: &str) -> i32 {
    let patterns = [
        Regex::new(r"(?i)S(\d{1,2})E\d{1,2}").unwrap(), // S01E02
        Regex::new(r"(?i)Season[ _]?(\d{1,2})").unwrap(), // Season 2
        Regex::new(r"(?i)S(\d{1,2})").unwrap(),         // S1
    ];

    for re in &patterns {
        if let Some(caps) = re.captures(item) {
            if let Some(season) = caps.get(1) {
                return season.as_str().parse().unwrap_or(0);
            }
        }
    }

    0
}

/// Extracts the episode number from a file or directory name.
/// Returns 0 if not found.
pub fn guess_episode(item: &str) -> i32 {
    let patterns = [
        Regex::new(r"(?i)S\d{1,2}E(\d{1,2})").unwrap(), // S01E02
        Regex::new(r"(?i)Episode[ _]?(\d{1,2})").unwrap(), // Episode 3
        Regex::new(r"(?i)E(\d{1,2})").unwrap(),         // E3
    ];

    for re in &patterns {
        if let Some(caps) = re.captures(item) {
            if let Some(ep) = caps.get(1) {
                return ep.as_str().parse().unwrap_or(0);
            }
        }
    }

    0
}

pub fn guess_name(item: &str) -> String {
    let quality_keywords = [
        "1080p", "720p", "bluray", "webrip", "web-dl", "hdrip", "x264", "x265", "hevc",
    ];
    let cleaned = item.replace(['.', '_', '[', ']', '(', ')'], " ");
    let tokens = cleaned.split_whitespace().collect::<Vec<_>>();

    let mut name_parts = Vec::new();

    for token in &tokens {
        if token.len() == 4 && token.chars().all(|c| c.is_ascii_digit()) {
            let y = token.parse::<u16>().unwrap_or(0);
            if (1900..=2099).contains(&y) {
                break;
            }
        }

        if quality_keywords
            .iter()
            .any(|q| token.eq_ignore_ascii_case(q))
        {
            break;
        }

        name_parts.push(*token);
    }

    name_parts.join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_guess_season() {
        assert_eq!(guess_season("S01E02"), 1);
        assert_eq!(guess_season("Season 2"), 2);
        assert_eq!(guess_season("S3"), 3);
        assert_eq!(guess_season("show.name.S10E05.1080p"), 10);
        assert_eq!(guess_season("some_folder/Season_12/"), 12);
        assert_eq!(guess_season("See - Season 1"), 1);
        assert_eq!(guess_season("The Pitt S01"), 1);
        assert_eq!(guess_season("Slow Horses SEASON 01 S01 COMPLETE"), 1);
    }

    #[test]
    fn test_guess_episode() {
        assert_eq!(guess_episode("S01E02"), 2);
        assert_eq!(guess_episode("Episode 3"), 3);
        assert_eq!(guess_episode("E04"), 4);
        assert_eq!(guess_episode("series.S10E05.1080p"), 5);
        assert_eq!(guess_episode("E99_something_else"), 99);
        assert_eq!(guess_episode("no_episode_here"), 0);
    }

    #[test]
    fn test_guess_name() {
        assert_eq!(
            guess_name("Breaking.Bad.S01E01.1080p.BluRay.x264"),
            "Breaking Bad"
        );
        assert_eq!(
            guess_name("The_Office_Season_3_Episode_2_HDTV"),
            "The Office Season 3 Episode 2 HDTV"
        );
        assert_eq!(
            guess_name("Friends.S05E10.720p.WEB-DL"),
            "Friends"
        );
        assert_eq!(
            guess_name("Some.Movie.1999.1080p.BluRay.x265"),
            "Some Movie"
        );
        assert_eq!(
            guess_name("No_Quality_Info_Here"),
            "No Quality Info Here"
        );
    }
}
