pub fn compare(t: char, r: char) -> bool {
    if t == r.to_lowercase().next().unwrap() {
        true
    } else if t == r {
        true
    } else if t == 'S' && r == 'ß' {
        true
    } else if t == 'U' && r == 'Ü' {
        true
    } else if t == 'A' && r == 'Ä' {
        true
    } else if t == 'O' && r == 'Ö' {
        true
    } else if t == 's' && r == 'ß' {
        true
    } else if t == 'u' && r == 'ü' {
        true
    } else if t == 'a' && r == 'ä' {
        true
    } else if t == 'o' && r == 'ö' {
        true
    } else if t == 's' && r == 'ß' {
        true
    } else if t == 'u' && r == 'Ü' {
        true
    } else if t == 'a' && r == 'Ä' {
        true
    } else if t == 'o' && r == 'Ö' {
        true
    } else if t == '"' && r == '«' {
        return true;
    } else if t == '"' && r == '»' {
        return true;
    } else if t == '-' && r == '–' {
        return true;
    } else {
        false
    }
}
