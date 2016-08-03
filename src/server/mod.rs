use prelude::*;
use std;

mod views;

fn msleep(ms: u64) {
	sleep(Duration::from_millis(ms));
}

pub trait Ext<'a> {
	fn ext<T: typemap::Key>(&'a self) -> &'a T::Value;
	fn ins<T: typemap::Key>(&mut self, val: T::Value);
}

impl<'a, 'b> Ext<'a> for Request<'a, 'b> {
	fn ext<T: typemap::Key>(&'a self) -> &'a T::Value {
		self.extensions.get::<T>().unwrap()
	}
	fn ins<T: typemap::Key>(&mut self, val: T::Value) {
		self.extensions.insert::<T>(val);
	}
}

pub enum Reply {
	Html(String),
	Redirect(String),
}
// let log = setup_logger(get_loglevel("SLOG_LEVEL"));
// let mainlog = log.new(o!["reqid" => "main"]);
// let worklog = log.new(o![]);
//
// defer!(trace![mainlog, "Clean exit"]);
// trace![mainlog, "Constructing middleware"];
//
// let mut chain = router;
// chain.link_before(Log::new(worklog));
// chain.link_before(Db);
// chain.link_around(ResponseTime);
// chain.link_after(Html);
//
// let mut mount = Mount::new();
// mount.mount("/", chain)
// .mount("/dl/", Static::new(Path::new("dl/")));
//
// trace![mainlog, "Starting server"];
// let _ = Iron::new(mount).http("localhost:3000").map_err(|x| {
// error![mainlog, "Unable to start server", "error" => format!("{:?}", x)];
// });
//

pub struct Html;
impl AfterMiddleware for Html {
	fn after(&self, req: &mut Request, mut res: Response) -> IronResult<Response> {
		trace![req.ext::<Log>(), "Setting MIME to html"];
		(Mime(TopLevel::Text, SubLevel::Html, vec![])).modify(&mut res);
		Ok(res)
	}
}

struct Head;
impl typemap::Key for Head {
	type Value = String;
}
impl BeforeMiddleware for Head {
	fn before(&self, req: &mut Request) -> IronResult<()> {
		let mut buffer = String::new();
		let _ = html! {
			buffer,
			head {
				meta charset="UTF-8" /
			}
		};
		req.ins::<Head>(buffer);
		Ok(())
	}
}

pub fn get_loglevel(env: &str) -> Level {
	macro_rules! lvlc {
		($n:expr, $($i:ident),*) => {{
			match $n {
				$(
					stringify!($i) => Level::$i,
				)*
				_ => Level::Info,
			}
		}};
	}
	match env::var(env) {
		Ok(val) => lvlc![&val[..], Trace, Debug, Info, Warning, Error],
		Err(_) => Level::Info,
	}
}

pub fn setup_logger(level: Level) -> Logger {
	let automatic = o!["line" => {
			|rec: &RecordInfo| {
				rec.line()
			}
		}, "mod" => {
			|rec: &RecordInfo| {
				rec.module().to_owned()
			}
		}];

	let log;
	if stderr_isatty() {
		log = drain::filter_level(level, ::slog_term::async_stderr()).into_logger(automatic);
		trace!(log, "Using drain", "out" => "stderr",
			"stderr_isatty" => stderr_isatty(),
			"type" => "term");
	} else {
		log = drain::filter_level(level,
		                          drain::async_stream(std::io::stderr(), ::slog_json::new()))
			.into_logger(automatic);
		trace!(log, "Using drain", "out" => "stderr",
			"stderr_isatty" => stderr_isatty(),
			"type" => "json");
	}
	log
}
