use ::reqwest::blocking::Client;
use graphql_client::{reqwest::post_graphql_blocking as post_graphql, GraphQLQuery};

#[allow(clippy::upper_case_acronyms)]
type DateTime = String;
type GitTimestamp = String;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "schema.docs.graphql",
    query_path = "query_pr_commits.graphql",
    response_derives = "Debug",
    normalization = "rust"
)]
struct RepoView;

fn main() {
    
    let github_api_token =
        std::env::var("SECRET_POA_GITHUB").expect("Missing SECRET_POA_GITHUB env var");

    let variables = repo_view::Variables {
        owner: "rust-lang".to_owned(),
        name: "rust".to_owned(),
    };

    let client = Client::builder()
        .user_agent("graphql-rust/0.10.0")
        .default_headers(
            std::iter::once((
                reqwest::header::AUTHORIZATION,
                reqwest::header::HeaderValue::from_str(&format!("Bearer {}", github_api_token))
                    .unwrap(),
            ))
            .collect(),
        )
        .build().unwrap();

    let response_body =
        post_graphql::<RepoView, _>(&client, "https://api.github.com/graphql", variables).unwrap();

    let response_data: repo_view::ResponseData = response_body.data.expect("missing response data");

    for pr in response_data
        .repository
        .expect("missing repository")
        .pull_requests
        .nodes
        .expect("pr nodes is null")
        .iter()
        .flatten()
    {

        //println!("PR: {:?}", pr);
        
        let mut commits = vec![];

        println!("---- PR({}): {}", pr.merged, pr.title);
        
        for commit in pr.commits.nodes.iter().flatten() {
            if let Some(commit) = commit {
                commits.push(commit.commit.abbreviated_oid.clone());
                println!(" {} - commit", commit.commit.abbreviated_oid);
            }
        }
        if let Some(nodes) = &pr.comments.nodes {
            for comment in nodes {
                if let Some(comment) = comment {
                    let author = match &comment.author {
                        Some(author) => author.login.to_owned(),
                        None => "".to_owned(),
                    };
                    if author == "bors" {
                        println!("{} - {}", author, comment.body);
                    }
                }
            }
        }
    }    
}
