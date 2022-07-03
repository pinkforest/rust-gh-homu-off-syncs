use ::reqwest::blocking::Client;
use graphql_client::{reqwest::post_graphql_blocking as post_graphql, GraphQLQuery};
//use std::time::Duration;
use std::time::SystemTime;

#[allow(clippy::upper_case_acronyms)]
type DateTime = String;
type GitTimestamp = String;
type GitObjectId = String;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "schema.docs.graphql",
    query_path = "query_pr_commits.graphql",
    response_derives = "Debug",
    normalization = "rust"
)]
struct RepoView;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "schema.docs.graphql",
    query_path = "query_more_commits.graphql",
    response_derives = "Debug",
    normalization = "rust"
)]
struct MoreCommits;

use lazy_static::lazy_static;
use regex::Regex;

fn main() {
    
    let github_api_token =
        std::env::var("SECRET_POA_GITHUB").expect("Missing SECRET_POA_GITHUB env var");

    lazy_static! {
        static ref RE_BORS_APPROVED: Regex = Regex::new(r"^\s*:pushpin: Commit ([A-F0-9a-f]+) has been approved").unwrap();
    }
    
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

    let mut pr_cursor: std::option::Option<String> = None;

    loop {

        //println!("Going for 50 PRs.. Cursor = {:?}", &pr_cursor);
        let sys_time = SystemTime::now();

        let real_cursor = pr_cursor.clone();
        
        let pr_variables = repo_view::Variables {
            owner: "rust-lang".to_owned(),
            name: "rust".to_owned(),
            pr_cursor: real_cursor,
        };
        
        let response_body =
            post_graphql::<RepoView, _>(&client, "https://api.github.com/graphql", pr_variables).unwrap();
        
        let response_data: repo_view::ResponseData = response_body.data.expect("missing response data");

        let mut prs = vec![];
        
        for pr_edge in response_data
            .repository
            .expect("missing repository")
            .pull_requests
            .edges
            .expect("pr edges is null")
            .iter()
            .flatten()
        {
            let pr = pr_edge.node.as_ref().unwrap();

            prs.push(format!("{}-{}", pr.number.clone(), pr_edge.cursor) );
            
            let merged_at = match &pr.merged_at {
                Some(m) => m.clone(),
                None => "N/A".to_owned(),
            };
            
            let mut last_commit   = "".to_owned();
            let mut last_approved = "".to_owned();
            
            for commit in pr.commits.nodes.iter().flatten() {
                if let Some(commit) = commit {
                    last_commit = commit.commit.oid.clone();
                }
            }

            let more_commit_variables = more_commits::Variables {
                pr_id: pr_node.id.clone(),
                commit_cursor: ..,
            };
            
            if let Some(nodes) = &pr.comments.nodes {
                for comment in nodes {
                    if let Some(comment) = comment {
                        let author = match &comment.author {
                            Some(author) => author.login.to_owned(),
                            None => "".to_owned(),
                        };
                        if author == "bors" {
                            if let Some(approved_commit_ref) = RE_BORS_APPROVED.captures_iter(&comment.body).next() {
                                last_approved = approved_commit_ref[1].to_string();
                            }
                        }
                    }
                }
            }
            if last_approved != last_commit {
                println!("#{} - {} - Approved {} != Last commit {}", pr.number, merged_at, last_approved, last_commit);
            }
            
            pr_cursor = Some(pr_edge.cursor.clone());
        }

        let iter_duration = sys_time
            .elapsed()
            .expect("Time went backwards");
        
        println!("Took: {:?} ms", iter_duration.as_millis());
        
    }
}
