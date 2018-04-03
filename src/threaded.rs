
// I want to take stdin, and read lines off of it, in a thread, and put them in a mpsc,
// and poll that from a stream, and return the lines.

use std::sync::mpsc;
use std::thread;
use std::io::{self, BufRead};

use futures::{Async, Future, Poll, Stream, StreamExt, FutureExt, Never};
use futures::task::Context;
use mio::{Evented, PollOpt, Poll as MioPoll, Ready, Registration as MioRegistration, Token};
use tokio::prelude::Future as OldFuture;
use tokio::reactor::{Registration as TokioRegistration, Handle};
use tokio::runtime::Runtime;

struct Stdin {
    rx: mpsc::Receiver<String>,
    registration: MioRegistration,
}

impl Stdin {
    pub fn new() -> Self {
        let (registration, set_readiness) = MioRegistration::new2();
        let (tx, rx) = mpsc::channel();
        thread::spawn(move || {
            let stdin = io::stdin();
            //let stdin = io::stdin().lock();
            for line in stdin.lock().lines() {
                tx.send(line.unwrap()).unwrap();
                set_readiness.set_readiness(Ready::readable());
            }
            eprintln!("exiting read thread");
        });
        Stdin { rx, registration }
    }
}

impl Evented for Stdin {
    fn register(&self, poll: &MioPoll, token: Token, interest: Ready, opts: PollOpt)
        -> io::Result<()>
    {
        self.registration.register(poll, token, interest, opts)
    }

    fn reregister(&self, poll: &MioPoll, token: Token, interest: Ready, opts: PollOpt)
        -> io::Result<()>
    {
        self.registration.reregister(poll, token, interest, opts)
    }

    fn deregister(&self, poll: &MioPoll) -> io::Result<()> {
        self.registration.deregister(poll)
    }
}

impl Stream for Stdin {
    type Item = String;
    type Error = io::Error;

    fn poll_next(&mut self, cx: &mut Context) -> Poll<Option<Self::Item>, Self::Error> {
        match self.rx.try_recv() {
            Ok(s) => Ok(Some(s).into()),
            Err(mpsc::TryRecvError::Empty) => Ok(Async::Pending),
            Err(mpsc::TryRecvError::Disconnected) => Ok(None.into()),
        }
    }
}

struct Core {
    stdin: Stdin,
    registration: TokioRegistration,
}

impl Core {
    pub fn new(handle: &Handle) -> Self {
        let stdin = Stdin::new();
        let registration = TokioRegistration::new();
        registration.register_with(&stdin, handle);
        Core { stdin, registration }
    }
}

impl Future for Core {
    type Item = ();
    type Error = Never;

    fn poll(&mut self, cx: &mut Context) -> Poll<Self::Item, Self::Error> {
        //let &mut Core { ref stdin, ref registration } = self;
        if let Async::Pending = self.registration.poll_read_ready2(cx).unwrap() {
            return Ok(Async::Pending);
        }
        loop {
            match self.stdin.poll_next(cx).unwrap() {
                Async::Pending => {
                    self.registration.poll_read_ready2(cx).unwrap();
                    return Ok(Async::Pending);
                },
                Async::Ready(Some(s)) => eprintln!("<<{}>>", s),
                Async::Ready(None) => panic!("ahhhh!"),
            }
        }
    }
}

pub fn main() {
    let mut rt = Runtime::new().unwrap();
    let core = Core::new(rt.reactor());
    rt.spawn2(core);
    rt.shutdown_on_idle().wait().unwrap();

}
