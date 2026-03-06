/// Parse timestamp chapters from a YouTube video description.
///
/// Handles common formats:
/// - `00:00 Title`
/// - `1:23:45 Title`
/// - `00:00 - Title`
/// - `[00:00] Title`
/// - `► 00:00 Title`
/// - `00:00 - Artist - Title`

#[derive(Debug, Clone)]
pub struct Chapter {
    pub timestamp: f64, // seconds
    pub title: String,
}

/// Parse chapters from a description string. Returns empty vec if no timestamps found.
pub fn parse_chapters(description: &str) -> Vec<Chapter> {
    let mut chapters = Vec::new();

    for line in description.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        if let Some(ch) = try_parse_line(trimmed) {
            chapters.push(ch);
        }
    }

    // Only return if we found at least 2 chapters (a single timestamp isn't a tracklist)
    if chapters.len() >= 2 {
        chapters
    } else {
        Vec::new()
    }
}

/// Find which chapter is currently playing at the given position.
pub fn current_chapter(chapters: &[Chapter], position: f64) -> Option<&Chapter> {
    if chapters.is_empty() {
        return None;
    }
    // Find the last chapter whose timestamp <= position
    let mut current = &chapters[0];
    for ch in chapters {
        if ch.timestamp <= position {
            current = ch;
        } else {
            break;
        }
    }
    Some(current)
}

fn try_parse_line(line: &str) -> Option<Chapter> {
    // Strip non-digit prefixes: ►, ▸, -, •, *, # etc.
    let stripped = line
        .trim_start_matches(|c: char| "►▸•*#→➤➜─".contains(c) || c.is_whitespace());

    // Try timestamp at start
    if let Some((secs, rest)) = extract_timestamp(stripped) {
        let title = clean_title(rest);
        if !title.is_empty() {
            return Some(Chapter { timestamp: secs, title });
        }
    }

    // Try timestamp at end of line (e.g., "Title - 03:22")
    if let Some((secs, _)) = extract_timestamp_end(line) {
        let title = clean_title_from_end(line);
        if !title.is_empty() {
            return Some(Chapter { timestamp: secs, title });
        }
    }

    None
}

fn extract_timestamp(s: &str) -> Option<(f64, &str)> {
    // Match optional [ bracket
    let s = s.strip_prefix('[').unwrap_or(s);

    let mut nums = Vec::new();
    let mut pos = 0;
    let chars: Vec<char> = s.chars().collect();

    loop {
        // Read digits
        let start = pos;
        while pos < chars.len() && chars[pos].is_ascii_digit() {
            pos += 1;
        }
        if pos == start {
            break;
        }
        let num_str: String = chars[start..pos].iter().collect();
        nums.push(num_str.parse::<f64>().ok()?);

        // Check for colon separator
        if pos < chars.len() && chars[pos] == ':' {
            pos += 1;
        } else {
            break;
        }
    }

    if nums.len() < 2 {
        return None;
    }

    let secs = match nums.len() {
        2 => nums[0] * 60.0 + nums[1],
        3 => nums[0] * 3600.0 + nums[1] * 60.0 + nums[2],
        _ => return None,
    };

    // Skip past optional ] bracket
    if pos < chars.len() && chars[pos] == ']' {
        pos += 1;
    }

    let byte_pos: usize = chars[..pos].iter().map(|c| c.len_utf8()).sum();
    let rest = &s[byte_pos..];

    Some((secs, rest))
}

fn extract_timestamp_end(line: &str) -> Option<(f64, &str)> {
    // Look for timestamp pattern near the end
    // Find last occurrence of digit:digit pattern
    let bytes = line.as_bytes();
    for i in (0..bytes.len().saturating_sub(4)).rev() {
        if bytes[i].is_ascii_digit() {
            if let Some((secs, _)) = extract_timestamp(&line[i..]) {
                return Some((secs, &line[..i]));
            }
        }
    }
    None
}

fn clean_title(s: &str) -> String {
    s.trim()
        .trim_start_matches(|c: char| c == '-' || c == '–' || c == '|' || c == ':' || c == '.' || c.is_whitespace())
        .trim_end_matches(|c: char| c == '-' || c == '–' || c == '|' || c.is_whitespace())
        .trim()
        .to_string()
}

fn clean_title_from_end(line: &str) -> String {
    // Remove the timestamp portion from the end
    let trimmed = line.trim();
    // Find where the timestamp starts by looking for the last separator before digits
    if let Some(sep_pos) = trimmed.rfind(|c: char| c == '-' || c == '–' || c == '|') {
        let before = trimmed[..sep_pos].trim();
        // Strip leading numbering like "1." or "01)"
        let cleaned = before
            .trim_start_matches(|c: char| c.is_ascii_digit() || c == '.' || c == ')' || c == '#')
            .trim();
        if !cleaned.is_empty() {
            return cleaned.to_string();
        }
    }
    String::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_timestamps() {
        let desc = "00:00 Intro\n01:30 First Song\n05:22 Second Song";
        let chapters = parse_chapters(desc);
        assert_eq!(chapters.len(), 3);
        assert_eq!(chapters[0].timestamp, 0.0);
        assert_eq!(chapters[0].title, "Intro");
        assert_eq!(chapters[1].timestamp, 90.0);
        assert_eq!(chapters[2].timestamp, 322.0);
    }

    #[test]
    fn test_with_separators() {
        let desc = "00:00 - Intro\n01:30 - First Song\n05:22 - Second Song";
        let chapters = parse_chapters(desc);
        assert_eq!(chapters.len(), 3);
        assert_eq!(chapters[0].title, "Intro");
    }

    #[test]
    fn test_hours() {
        let desc = "0:00:00 Start\n1:23:45 Middle\n2:00:00 End";
        let chapters = parse_chapters(desc);
        assert_eq!(chapters.len(), 3);
        assert_eq!(chapters[1].timestamp, 5025.0);
    }

    #[test]
    fn test_with_arrows() {
        let desc = "► 00:00 Track One\n► 03:15 Track Two\n► 07:30 Track Three";
        let chapters = parse_chapters(desc);
        assert_eq!(chapters.len(), 3);
        assert_eq!(chapters[0].title, "Track One");
    }

    #[test]
    fn test_bracketed() {
        let desc = "[00:00] Intro\n[03:15] Track Two\n[07:30] Track Three";
        let chapters = parse_chapters(desc);
        assert_eq!(chapters.len(), 3);
        assert_eq!(chapters[0].title, "Intro");
    }

    #[test]
    fn test_current_chapter() {
        let chapters = vec![
            Chapter { timestamp: 0.0, title: "Intro".into() },
            Chapter { timestamp: 90.0, title: "First".into() },
            Chapter { timestamp: 300.0, title: "Second".into() },
        ];
        assert_eq!(current_chapter(&chapters, 50.0).unwrap().title, "Intro");
        assert_eq!(current_chapter(&chapters, 90.0).unwrap().title, "First");
        assert_eq!(current_chapter(&chapters, 200.0).unwrap().title, "First");
        assert_eq!(current_chapter(&chapters, 400.0).unwrap().title, "Second");
    }

    #[test]
    fn test_single_timestamp_ignored() {
        let desc = "Check out 03:15 for the drop";
        let chapters = parse_chapters(desc);
        assert!(chapters.is_empty());
    }
}
