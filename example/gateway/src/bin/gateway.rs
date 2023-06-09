use axum::{
    body::{Body, Bytes},
    // async_trait,
    extract::Query,
    handler::{get, post},
    http::header::HeaderMap,
    http::Request,
    response::IntoResponse,
    Router,
};

use serde::Deserialize;
use std::net::SocketAddr;
use tower::BoxError;

use hex;
use std::io::prelude::*;
use std::path::Path;
use std::process::{Command, Stdio};

use gateway::database::mysql;
use gateway::pack::packfile::{Packfile, packfile_read, get_ofs_delta, get_hash};
use sqlx::mysql::{MySqlPoolOptions, MySqlConnection};


#[tokio::main]
async fn main() {
    // initialize tracing
    tracing_subscriber::fmt::init();

    // build our application with a route
    let app = Router::new()
        // route("/", post(git_receive_pack_2)).
        .route("/test.git/info/refs", get(handle_refs))
        .route("/test.git/git-receive-pack", post(process_pack));

    // run our app with hyper
    // `axum::Server` is a re-export of `hyper::Server`
    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    tracing::debug!("listening on {}", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

#[derive(Deserialize)]
struct ServiceName {
    service: String,
}

// async fn git_receive_pack(Query(service): Query<ServiceName>) -> String {
//     println!("{}", service.service);
//     service.service
// }
async fn handle_refs(
    Query(service): Query<ServiceName>,
    _context: Request<Body>,
) -> impl IntoResponse {
    let mut headers = HeaderMap::new();
    let mode = format!("application/x-{}-advertisement", service.service);

    headers.insert(
        "Cache-Control",
        "no-cache, max-age=0, must-revalidate".parse().unwrap(),
    );
    headers.insert("Content-Type", mode.parse().unwrap());
    headers.insert("Expires", "Fri, 01 Jan 1980 00:00:00 GMT".parse().unwrap());

    headers.insert("Pragma", "no-cache".parse().unwrap());

    // println!("{:?}", service.service);
    // println!("{:?}", context.headers());
    if service.service != "git-receive-pack" && service.service != "git-upload-pack" {
        // return "Operation not permitted！！！".as_bytes();
        return (headers, String::from("Operation not permitted！！！"));
    }
    let repo_path = "/root/Tmp/repositories/test.git";
    Command::new("git")
        .args(["init", "--bare", repo_path])
        .output()
        .expect("sh exec error!");

    let path = Path::new(&repo_path);
    if !path.exists() {
        return (headers, String::from("Not Found!"));
    }
    let mut response_body = String::from("001f# service=git-receive-pack\n0000");
    let refs_bytes = Command::new("git") // 自己检查
        .args([
            "receive-pack",
            "--stateless-rpc",
            "--advertise-refs",
            repo_path,
        ])
        .output()
        .expect("sh exec error!");
    // println!("{:?}", refs_bytes);

    let output = String::from_utf8(refs_bytes.stdout).unwrap();

    response_body = response_body + output.as_str(); // output 检查本地和服务器数据的不同 返回不同的引用
                                                     // println!("{}", response_body);

    (headers, response_body)
}
async fn process_pack(req: Request<Body>) -> impl IntoResponse {
    let repo_path = "/root/Tmp/repositories/test.git";

    // 拦截 解析 pack
    let (_parts, body) = req.into_parts();
    let mut bytes = buffer_and_print("request", body).await.unwrap();

    // bytes
    let mut pipe = Command::new("git")
        .args(["receive-pack", "--stateless-rpc", repo_path])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("Failed to spawn child process");

    read_body(&mut bytes).await.unwrap();

    let mut stdin = pipe.stdin.take().expect("Failed to open stdin");
    std::thread::spawn(move || {
        stdin
            .write_all(&mut bytes)
            .expect("Failed to write to stdin");
    });

    let output = pipe.wait_with_output().expect("Failed to read stdout");

    // // response
    // println!("{}", std::str::from_utf8(&output.stdout).unwrap());
    // println!("{:?}", std::str::from_utf8(&output.stdout));

    output.stdout
    // let response = "0023\u{2}Resolving deltas:   0% (0/74)\r0041\u{2}Resolving deltas:   1% (1/74)\rResolving deltas:   2% (2/74)\r0023\u{2}Resolving deltas:   4% (3/74)\r0023\u{2}Resolving deltas:   5% (4/74)\r0023\u{2}Resolving deltas:   6% (5/74)\r0023\u{2}Resolving deltas:   8% (6/74)\r0023\u{2}Resolving deltas:   9% (7/74)\r0023\u{2}Resolving deltas:  10% (8/74)\r0024\u{2}Resolving deltas:  13% (10/74)\r0024\u{2}Resolving deltas:  14% (11/74)\r0024\u{2}Resolving deltas:  16% (12/74)\r0043\u{2}Resolving deltas:  18% (14/74)\rResolving deltas:  20% (15/74)\r0085\u{2}Resolving deltas:  21% (16/74)\rResolving deltas:  24% (18/74)\rResolving deltas:  25% (19/74)\rResolving deltas:  27% (20/74)\rReso003f\u{2}lving deltas:  28% (21/74)\rResolving deltas:  31% (23/74)\r0024\u{2}Resolving deltas:  32% (24/74)\r0043\u{2}Resolving deltas:  35% (26/74)\rResolving deltas:  36% (27/74)\r0024\u{2}Resolving deltas:  37% (28/74)\r0024\u{2}Resolving deltas:  40% (30/74)\r0024\u{2}Resolving deltas:  41% (31/74)\r0024\u{2}Resolving deltas:  43% (32/74)\r0024\u{2}Resolving deltas:  44% (33/74)\r0024\u{2}Resolving deltas:  45% (34/74)\r0043\u{2}Resolving deltas:  47% (35/74)\rResolving deltas:  48% (36/74)\r0024\u{2}Resolving deltas:  50% (37/74)\r0024\u{2}Resolving deltas:  51% (38/74)\r0024\u{2}Resolving deltas:  52% (39/74)\r0024\u{2}Resolving deltas:  54% (40/74)\r0024\u{2}Resolving deltas:  55% (41/74)\r0024\u{2}Resolving deltas:  56% (42/74)\r0024\u{2}Resolving deltas:  58% (43/74)\r0024\u{2}Resolving deltas:  59% (44/74)\r0024\u{2}Resolving deltas:  60% (45/74)\r0024\u{2}Resolving deltas:  63% (47/74)\r0024\u{2}Resolving deltas:  64% (48/74)\r0024\u{2}Resolving deltas:  66% (49/74)\r0024\u{2}Resolving deltas:  67% (50/74)\r0043\u{2}Resolving deltas:  68% (51/74)\rResolving deltas:  70% (52/74)\r0043\u{2}Resolving deltas:  71% (53/74)\rResolving deltas:  72% (54/74)\r0024\u{2}Resolving deltas:  74% (55/74)\r0043\u{2}Resolving deltas:  75% (56/74)\rResolving deltas:  77% (57/74)\r0024\u{2}Resolving deltas:  79% (59/74)\r0024\u{2}Resolving deltas:  81% (60/74)\r0024\u{2}Resolving deltas:  82% (61/74)\r0024\u{2}Resolving deltas:  83% (62/74)\r0024\u{2}Resolving deltas:  85% (63/74)\r0024\u{2}Resolving deltas:  86% (64/74)\r0024\u{2}Resolving deltas:  87% (65/74)\r0024\u{2}Resolving deltas:  90% (67/74)\r0024\u{2}Resolving deltas:  91% (68/74)\r0024\u{2}Resolving deltas:  94% (70/74)\r0024\u{2}Resolving deltas:  95% (71/74)\r0024\u{2}Resolving deltas:  97% (72/74)\r0024\u{2}Resolving deltas:  98% (73/74)\r0024\u{2}Resolving deltas: 100% (74/74)\r002b\u{2}Resolving deltas: 100% (74/74), done.\n0030\u{1}000eunpack ok\n0019ok refs/heads/master\n00000000";
    // println!("{}", response);
    // response.as_bytes()
}
async fn buffer_and_print<B>(direction: &str, body: B) -> Result<Bytes, BoxError>
where
    B: axum::body::HttpBody<Data = Bytes>,
    B::Error: Into<BoxError>,
{
    let bytes = hyper::body::to_bytes(body).await.map_err(Into::into)?;
    if let Ok(body) = std::str::from_utf8(&bytes) {
        tracing::debug!("{} body = {:?}", direction, body);
    }
    Ok(bytes)
}
async fn read_body(body: &mut Bytes) -> Result<(), sqlx::Error> {
    let mut index = 0;
    if &body[index..index + 4] != b"0000" {
        let (context, len) = read_line(body, index);
        println!("{}\n{}", context, len);
        index += len;
    }

    let packfile = Packfile::new(body[index + 4..].to_vec()).unwrap();
    let database_url = "mysql://root:123456@localhost:3306/git";

    let pool = MySqlPoolOptions::new()
            .max_connections(5)
            .connect(database_url)
            .await?;
    let mut conn = pool.acquire().await?;

    let objects = packfile.objects;
    let mut len = objects.len();
    
    while len >0 {
        len -= 1;
        let elem = objects.get(len).unwrap();
        // let data:Vec<u8> =  elem.data;
        let mut git_index = mysql::GitIndex {
            sha_1: Some(elem.hash.clone()),
            obj_type: elem.meta_info.obj_type,
            size: elem.meta_info.size,
            size_in_packfile: elem.size_in_packfile,
            offset_in_pack: elem.offset,
            depth: elem.depth,
            base_sha_1: Some(elem.base_sha_1.clone()),
        };
        mysql::insert(&mut git_index,&mut conn).await?;

        match elem.meta_info.obj_type {

            0..=6 => {
                let mut sha_1 = &mut git_index.sha_1.clone().unwrap();


                mysql::insert_blob(&mut sha_1, elem.content.clone(), &mut conn).await?;
            }
            
            _ => {
                let pack = &mut body[index + 4..].to_vec();

                get_ref_delta(pack,&mut git_index, &mut elem.data.clone(),&mut conn).await?;
            

            }
        }

    }
    // for mut elem in packfile.objects {

    //     // let data:Vec<u8> =  elem.data;
    //     let mut git_index = mysql::GitIndex {
    //         sha_1: Some(elem.hash),
    //         obj_type: elem.meta_info.obj_type,
    //         size: elem.meta_info.size,
    //         size_in_packfile: elem.size_in_packfile,
    //         offset_in_pack: elem.offset,
    //         depth: elem.depth,
    //         base_sha_1: Some(elem.base_sha_1),
    //     };
    //     mysql::insert(&mut git_index,&mut conn).await?;

    //     match elem.meta_info.obj_type {
    //         0..=6 => {
    //             let mut sha_1 = &mut git_index.sha_1.clone().unwrap();


    //             mysql::insert_blob(&mut sha_1, elem.content, &mut conn).await?;
    //         }
    //         _ => {
    //             let pack = &mut body[index + 4..].to_vec();

                
    //             get_ref_delta(pack,&mut git_index, &mut elem.data,&mut conn).await?;
            

    //         }
    //     }
    // }

  
    println!("end");

    Ok(())
}
fn read_line(body: &mut Bytes, index: usize) -> (String, usize) {
    let line_len_str: String = format!("{:x?}", &body.slice(index..index + 4));

    let line_len = &hex::decode(&line_len_str[2..6]).unwrap();
    let len = line_len[0] as usize * 256 + line_len[1] as usize;
    if len == 0 {
        return ("".to_string(), 0);
    }
    let slice_context = &body.slice(index..len);

    let context = std::str::from_utf8(slice_context).unwrap();

    // println!("{}", context);
    (String::from(context), len)
}
async fn get_ref_delta(pack: &mut Vec<u8>, git_index:&mut mysql::GitIndex, data: &mut Vec<u8>, conn: &mut MySqlConnection) -> Result<(), sqlx::Error>{

    let mut base_sha_1 = git_index.base_sha_1.clone().unwrap();
    println!("{}",base_sha_1);

    let base_index: mysql::GitIndex = mysql::get_index(&mut base_sha_1, conn).await?;
    println!("base_index:{:?}", base_index);

    let mut offset_in_pack = base_index.offset_in_pack as usize;
    
    let object = packfile_read(pack,&mut offset_in_pack).unwrap();
    println!("data:{:?}, instr:{:?}", object.data,data);

    let (mut result, _written) = get_ofs_delta(object.data, data);
    git_index.sha_1 = Some(get_hash(git_index.obj_type,&mut result).unwrap());
    
    mysql::insert(git_index, conn).await?;

    let sha_1 = &mut git_index.sha_1.clone().unwrap();
    mysql::insert_blob(sha_1, std::str::from_utf8(&result).unwrap().to_string(), conn).await?;

    Ok(())
}