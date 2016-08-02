use include::*;
mod views;

fn msleep(ms: u64) {
	sleep(Duration::from_millis(ms));
}

trait Db {
	fn log(&self) -> &Arc<Logger>;
}
impl<'a, 'b> Db for Request<'a, 'b> {
	fn log(&self) -> &Arc<Logger> {
		self.extensions.get::<Log>().unwrap()
	}
}

pub fn enter() {

	let router = req! {

		get "/", myfun: (req, ext) => {
			msleep(1000);
			trace![ext.log, "Nice"];
			Ok(Response::with((status::Ok, views::index(req.log()))))
		},

		get "/other/:test", kek: (req, _) => {
			trace![elog!(req), "other route"];
			msleep(1000);
			trace![req.log(), "cool", "req" => format!("{:?}", req.extensions.get::<Router>().unwrap().find("test"))];
			Ok(Response::with((status::Ok, "Hello World")))
		},

		get "/*", some: (req, _) => {
			msleep(1000);
			warn![elog!(req), "Unknown route", "req" => format!("{:?}", req)];
			Ok(Response::with((status::Found, Header(
				headers::Location(
					"other".to_owned()

				)
			))))
		},

	};

	let log = setup_logger(get_loglevel("SLOG_LEVEL"));
	let mainlog = log.new(o!["reqid" => "main"]);
	let worklog = log.new(o![]);

	defer!(trace![mainlog, "Clean exit"]);
	trace![mainlog, "Constructing middleware"];

	let mut chain = Chain::new(router);
	chain.link_before(Log::new(worklog));
	chain.link_around(ResponseTime);
	chain.link_after(Html);

	let mut mount = Mount::new();
	mount
		.mount("/", chain)
		.mount("/dl/", Static::new(Path::new("dl/")))
	;

	trace![mainlog, "Starting server"];
	let _ = Iron::new(mount).http("localhost:3000").map_err(|x| {
		error![mainlog, "Unable to start server", "error" => format!("{:?}", x)];
	});
}

struct Html;
impl AfterMiddleware for Html {
	fn after(&self, req: &mut Request, mut res: Response) -> IronResult<Response> {
		trace![elog!(req), "Setting MIME to html"];
		(Mime(TopLevel::Text, SubLevel::Html, vec![])).modify(&mut res);
		Ok(res)
	}
}

struct Log(Arc<Logger>, Mutex<u64>);

impl Log {
	fn new(log: Logger) -> Log {
		Log(Arc::new(log), Mutex::new(0))
	}
}

impl typemap::Key for Log { type Value = Arc<Logger>; }

impl BeforeMiddleware for Log {
	fn before(&self, req: &mut Request) -> IronResult<()> {
		let reqid = {
			let mut count = self.1.lock().unwrap();
			*count = count.wrapping_add(1);
			*count
		};
		ins!(req, Log: Arc::new(self.0.new(o!["reqid" => reqid])));
		trace![elog!(req), "Beginning request"];
		Ok(())
	}
}

struct Head;
impl typemap::Key for Head { type Value = String; }
impl BeforeMiddleware for Head {
	fn before(&self, req: &mut Request) -> IronResult<()> {
		let mut buffer = String::new();
		html! {
			buffer,
			head {
				meta charset="UTF-8" /
			}
		};
		ins!(req, Head: buffer);
		Ok(())
	}
}

struct BodyWrap;

struct ResponseTime;
impl AroundMiddleware for ResponseTime {
	fn around(self, handler: Box<Handler>) -> Box<Handler> {
		Box::new(ResponseTimeHandler(handler))
	}
}

struct ResponseTimeHandler(Box<Handler>);
impl Handler for ResponseTimeHandler {
	fn handle(&self, req: &mut Request) -> IronResult<Response> {
		let begin = precise_time_ns();
		let response = self.0.handle(req);
		let delta = precise_time_ns() - begin;
		let conn = Connection::connect("postgresql://postgres:abc@localhost/hybrida", SslMode::None)
			.map_err(|x| {
				crit![elog!(req), "Unable to connec to db", "error" => format!("{:?}", x)];
			});
		if let Ok(conn) = conn {
			let _ = conn.transaction();
		}

		trace!(elog!(req), "Request time",
			"ms" => delta / 1000 / 1000, "us" => delta / 1000 % 1000, "ns" => delta % 1000
		);

		response
	}
}

fn get_loglevel(env: &str) -> Level {
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
		Ok(val) => {
			lvlc![&val[..], Trace, Debug, Info, Warning, Error]
		}
		Err(_) => Level::Info,
	}
}

fn setup_logger(level: Level) -> Logger {
	if stderr_isatty() {
		let log = slog_term::async_stderr().into_logger(o![]);
		trace!(log, "Using drain", "out" => "stderr", "stderr_isatty" => stderr_isatty(), "type" => "term");
		log
	} else {
		let log = drain::stream(
			std::io::stderr(),
			slog_json::new()
			).into_logger(o![]);
		trace!(log, "Using drain", "out" => "stderr", "stderr_isatty" => stderr_isatty(), "type" => "json");
		log
	}
}
