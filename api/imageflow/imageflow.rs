#![allow(dead_code)]
use http::StatusCode;
use now_lambda::{lambda, error::NowError, IntoResponse, Request, Response};
use std::error::Error;
use url;
use url::form_urlencoded;
use regex;
use s3::bucket::Bucket;
use s3::credentials::Credentials;


use imageflow_core::errors::FlowError;
use std::env;


// Start the runtime with the handler
fn main() -> Result<(), Box<dyn Error>> {
	Ok(lambda!(handler))
}

fn handler(req: Request) -> Result<impl IntoResponse, NowError> {

	let query = req.uri().query().unwrap_or("");
	let path = get_query_value(query, "imageflow_path");

	if let Some((region, bucket, path)) = parse_s3_path(&path) {
		return proxy_image_s3(region, bucket, path, query.to_owned());
	}
	let response = Response::builder()
		.status(StatusCode::OK)
		.header("Content-Type", "text/plain")
		.body(format!("Recieved path {}\n\
					To resize an S3 Image, use /api/imageflow/s3/[region]/[bucket]/path?width=200\n\
					Example: /api/imageflow/s3/us-west-2/imageflow-resources/test_inputs/u1.jpg?width=400",path).as_bytes().to_vec())
		.expect("Internal Server Error");

		Ok(response)
}

fn get_query_value(query: &str, key: &str) -> String{
	let pairs = form_urlencoded::parse(query.as_bytes());
	pairs.into_iter()
		.filter(|(k,_)| k == key)
		.map( |(_, v)| v.into_owned())
		.next()
		.unwrap_or("".to_owned())
}

fn parse_s3_path(path: &str) -> Option<(String, String, String)>{
	let re = regex::Regex::new(r"s3/(?P<region>[a-z0-9-]+)/(?P<bucket>[a-z0-9-.]+)/(?P<path>.*)").unwrap();
	if let Some(caps) = re.captures(path) {
		let region = &caps["region"];
		let bucket = &caps["bucket"];
		let path = &caps["path"];
		Some((region.to_owned(), bucket.to_owned(),path.to_owned()))
	}else{
		None
	}
}

fn process_image(input: &[u8], query: String) -> Result<(Vec<u8>, String), FlowError>{
	let mut job = imageflow_core::Context::create_can_panic()?;

	job.add_input_bytes(0, input)?;
	job.add_output_buffer(1)?;
	let response = job.execute_1(imageflow_types::Execute001 {
		graph_recording: None,
		framewise: imageflow_types::Framewise::Steps(
			vec![
				imageflow_types::Node::CommandString {
					kind: imageflow_types::CommandStringKind::ImageResizer4,
					value: query,
					decode: Some(0),
					encode: Some(1)
				}
			]
		)
	})?;
	if let imageflow_types::ResponsePayload::JobResult(result) = response {
		let mime = result.encodes.first().unwrap().preferred_mime_type.to_owned();
		let bytes = job.get_output_buffer_slice(1)?.to_vec();
		Ok((bytes, mime))
	} else {
		panic!("");
	}
}

fn proxy_image_s3(region: String, bucket: String, path: String, query: String) -> Result<http::response::Response<Vec<u8>>, NowError> {
	// for (k, v) in env::vars(){
	// 	eprintln!("{}={}", k, v);
	// }
	let access_key = env::var("IMAGEFLOW_AWS_ACCESS_KEY_ID").expect("Missing env var IMAGEFLOW_AWS_ACCESS_KEY_ID");
	let secret_key = env::var("IMAGEFLOW_AWS_ACCESS_KEY_SECRET").expect("Missing env var IMAGEFLOW_AWS_ACCESS_KEY_SECRET");


	let creds = Credentials::new_blocking(Some(access_key), Some(secret_key), None, None).unwrap();

	let bucket_obj = Bucket::new(&bucket, region.parse().unwrap(), creds)
		.map_err(|e| NowError::new(&format!("{}",e)))?;
	let (data, code) = bucket_obj.get_object_blocking(&path)
		.map_err(|e| NowError::new(&format!("{}",e)))?;
	if code < 200 || code >= 300 {
		return Err(NowError::new(&format!("Upstream HTTP error {} for S3 region {} bucket {} path {}", code, region, bucket, &path)));
	}

	let (bytes, mime) = process_image(&data, query)
		.map_err(|e| NowError::new(&format!("{}",e)))?;

	let response = Response::builder()
		.status(StatusCode::OK)
		.header("Content-Type", mime)
		.body(bytes)
		.expect("Internal Server Error");

	Ok(response)

}