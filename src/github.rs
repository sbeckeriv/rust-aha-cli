#[derive(Debug, Clone)]
pub struct GithubEnv {
    pub github_api_token: String,
    pub workflow_repo: String,
    pub workflow_login: String,
    pub silent: bool,
    pub verbose: bool,
}

#[derive(Debug)]
pub struct PullRequest {
    pub number: i64,
    pub name: String,
    pub url: String,
    pub labels: Vec<String>,
}

fn parse_repo_name(repo_name: &str) -> Result<(&str, &str), failure::Error> {
    let mut parts = repo_name.split('/');
    match (parts.next(), parts.next()) {
        (Some(owner), Some(name)) => Ok((owner, name)),
        _ => Err(format_err!("wrong format for the repository name param (we expect something like facebook/graphql/(optional name if not we use the org)"))
    }
}

pub fn pr_table(response_body: &RootInterface) {
    let mut table = prettytable::Table::new();
    for issue in response_body.items.iter() {
        let ref_head = issue.title.clone();
        let label_names: Vec<String> = issue
            .labels
            .iter()
            .map(|label| label.name.clone())
            .collect();
        let mut body = issue.body.clone();

        let checked = body.contains(
            "- [x] [Correctness](https://big.aha.io/products/ENG/pages/ENG-N-53)
- [x] [Readability](https://big.aha.io/products/ENG/pages/ENG-N-53)
- [x] [Security](https://big.aha.io/products/ENG/pages/ENG-N-103)
- [x] [Testing](https://big.aha.io/products/ENG/pages/ENG-N-53)",
        );

        body.truncate(20);
        // let requested_reviewers: Vec<String> = issue .requested_reviewers .iter() .map(|user| user.login.clone()) .collect();

        let mergeable = match &issue.mergeable {
            Some(mer) => mer.clone(),
            _ => "".to_string(),
        };

        let mergeable_state = match &issue.mergeable_state {
            Some(mer) => mer.clone(),
            _ => "".to_string(),
        };
        table.add_row(row!(
            issue.title,
            body,
            ref_head,
            label_names.join(","),
            issue.html_url,
            checked,
            mergeable,
            mergeable_state,
            //requested_reviewers.join(",")
        ));
    }
    table.printstd();
}

pub fn pr_data(config: &GithubEnv, author: String, open: bool) -> RootInterface {
    let (owner, name) = parse_repo_name(&config.workflow_repo).unwrap();
    let client = reqwest::Client::new();
    let state = if open { "open" } else { "closed" };
    let author = if author.len() > 0 {
        format!("+author:{}", author)
    } else {
        author
    };
    let url = format!(
        "https://api.github.com/search/issues?q=is:{}+is:pr+repo:{}/{}{}&sort=created",
        state, owner, name, author
    );
    if config.verbose {
        println!("github search url: {}", url)
    }
    let mut res = client
        .get(&url)
        .basic_auth(
            config.workflow_login.clone(),
            Some(config.github_api_token.clone()),
        )
        .send()
        .unwrap();

    res.json().expect("Could not find repo")
}

pub fn prs(config: GithubEnv) -> Result<Vec<PullRequest>, failure::Error> {
    let response_body = pr_data(&config, config.workflow_login.clone(), true);
    if config.verbose {
        pr_table(&response_body);
    }
    let response_data = response_body.items;
    let mut branches: Vec<PullRequest> = Vec::new();
    for issue in &response_data {
        let ref_head = issue.title.clone();
        let label_names: Vec<String> = issue
            .labels
            .iter()
            .map(|label| label.name.clone())
            .collect();
        let pull = PullRequest {
            number: issue.number,
            url: issue.html_url.clone(),
            name: ref_head,
            labels: label_names,
        };
        branches.push(pull);
    }

    Ok(branches)
}

#[derive(Serialize, Debug, Deserialize, Clone)]
struct Items {
    url: String,
    html_url: String,
    id: i64,
    node_id: String,
    number: i64,
    title: String,
    labels: Vec<Labels>,
    state: String,
    created_at: String,
    updated_at: String,
    closed_at: Option<String>,
    body: String,
    mergeable: Option<String>,
    mergeable_state: Option<String>,
    requested_reviewers: Option<Vec<User>>,
}

#[derive(Serialize, Debug, Deserialize, Clone)]
struct Labels {
    name: String,
}

#[derive(Serialize, Debug, Deserialize)]
struct PullRequest1 {
    url: String,
    html_url: String,
    diff_url: String,
    patch_url: String,
}

#[derive(Serialize, Debug, Deserialize)]
pub struct RootInterface {
    total_count: i64,
    incomplete_results: bool,
    items: Vec<Items>,
}

#[derive(Serialize, Debug, Deserialize, Clone)]
struct User {
    login: String,
    id: i64,
    node_id: String,
    avatar_url: String,
    gravatar_id: String,
    url: String,
    html_url: String,
    followers_url: String,
    following_url: String,
    gists_url: String,
    starred_url: String,
    subscriptions_url: String,
    organizations_url: String,
    repos_url: String,
    events_url: String,
    received_events_url: String,
    #[serde(rename = "type")]
    _type: String,
    site_admin: bool,
}
