use scraper::Selector;
use serde::Serialize;

#[derive(Debug, Default, Clone, PartialEq, Serialize)]
pub struct Article {
    pub translated: String,
    pub title: String,
    pub paragraphs: Vec<Paragraph>,
}

#[derive(Debug, Default, Clone, PartialEq, Serialize)]
pub struct Paragraph {
    pub original: String,
    pub translation: Option<String>,
    pub pairs: Option<Vec<Pair>>,
}

#[derive(Debug, Default, Clone, PartialEq, Serialize)]
pub struct Pair {
    pub original: Vec<usize>,
    pub translation: Vec<usize>,
}

impl Article {
    pub fn from_str(title: String, subtitle: String, paragraphs: Vec<String>) -> Self {
        let paragraphs: Vec<Paragraph> = paragraphs
            .into_iter()
            .map(|original| Paragraph {
                original,
                translation: None,
                pairs: None,
            })
            .collect();

        Article {
            translated: "false".to_string(),
            title: format!("{title} || {subtitle}"),
            paragraphs,
        }
    }
}

pub fn extract_article(
    client: &reqwest::blocking::Client,
    site: &String,
    access_info: &String,
) -> anyhow::Result<Article> {
    println!("-------------------------------");
    println!("processing site {site}");
    let response = client.get(site);
    let response = response
        .header("Cookie", format!("accessInfo={access_info}"))
        .header(
            "User-Agent",
            "Mozilla/5.0 (X11; Linux x86_64; rv:129.0) Gecko/20100101 Firefox/129.0",
        );
    let response = response.send()?.text()?;
    let document = scraper::Html::parse_document(&response);
    let title = Selector::parse(":is(span.font-brandUI:nth-child(2), .font-serifdisplayUI) > span")
        .unwrap();
    let title = document
        .select(&title)
        .flat_map(|item| item.children())
        .filter(|item| item.value().is_text())
        .map(|item| item.value().as_text().unwrap())
        .map(|item| item.trim().to_string())
        .filter(|item| !item.is_empty())
        .collect::<Vec<String>>()
        .join("\n");

    let subtitle = Selector::parse("div.RichText.leading-loose").unwrap();
    let subtitle = document
        .select(&subtitle)
        .flat_map(|item| item.children())
        .filter_map(|fragment| {
            if fragment.value().is_text() {
                Some(fragment)
            } else if fragment.has_children() {
                Some(fragment.first_child().unwrap())
            } else {
                None
            }
        })
        .map(|item| item.value().as_text().unwrap())
        .map(|item| item.trim())
        .collect::<String>();

    let body = Selector::parse("div[data-sara-click-el=body_element] > div.RichText > :is(p, h3)")
        .unwrap();
    let paragraphs = document
        .select(&body)
        .map(|item| {
            item.children()
                .filter_map(|fragment| {
                    if fragment.value().is_text() {
                        Some(fragment)
                    } else if fragment.has_children() {
                        Some(fragment.first_child().unwrap())
                    } else {
                        None
                    }
                })
                //.map(|item| {
                //    println!("{:?}", item.value().as_text());
                //    item
                //})
                .filter_map(|item| item.value().as_text())
                .map(|item| item.to_string())
                .map(clean)
                .collect::<String>()
        })
        .collect::<Vec<String>>();
    println!("-------------------------------");

    Ok(Article::from_str(clean(title), clean(subtitle), paragraphs))
}

fn clean(s: String) -> String {
    s.chars()
        .map(|c| if c == '–' { '-' } else { c })
        .map(|c| if c == '»' || c == '«' { '\"' } else { c })
        .filter(|c| {
            c.is_alphabetic() || c.is_numeric() || c.is_whitespace() || c.is_ascii_punctuation()
        })
        .collect()
}
