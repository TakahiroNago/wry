use objc2::{class, msg_send, runtime::*, sel};
use objc2_foundation::{NSPoint, NSRect, NSSize};
use raw_window_handle::*;
use std::{ffi::c_void, ptr::NonNull};
use tao::{event::{Event, StartCause}, event_loop::{ControlFlow, EventLoop}, platform::macos::WindowExtMacOS, window::WindowBuilder};
use wry::{dpi::*, Rect, WebView, WebViewBuilder, WebViewExtMacOS};

type Obj = *mut AnyObject;
struct H(Obj);
impl HasWindowHandle for H {
  fn window_handle(&self) -> Result<WindowHandle<'_>, HandleError> {
    let h = AppKitWindowHandle::new(NonNull::new(self.0 as *mut c_void).unwrap());
    Ok(unsafe { WindowHandle::borrow_raw(RawWindowHandle::AppKit(h)) })
  }
}
extern "C-unwind" fn yes(_: *mut AnyObject, _: Sel) -> Bool { Bool::YES }
extern "C-unwind" fn two(_: *mut AnyObject, _: Sel) -> f64 { 2.0 }
fn l(x: i32, y: i32, w: i32, h: i32) -> Rect {
  Rect { position: LogicalPosition::new(x, y).into(), size: LogicalSize::new(w, h).into() }
}
fn p(x: i32, y: i32, w: i32, h: i32) -> Rect {
  Rect { position: PhysicalPosition::new(x, y).into(), size: PhysicalSize::new(w, h).into() }
}
// Geometry taken from AppKit in window coordinates (y up), so it does not depend on isFlipped.
unsafe fn show(label: &str, wv: &WebView) {
  let w = wv.webview();
  let v = &*w as *const _ as Obj;
  let parent: Obj = msg_send![v, superview];
  let null: Obj = std::ptr::null_mut();
  let vb: NSRect = msg_send![v, bounds];
  let pb: NSRect = msg_send![parent, bounds];
  let c: NSRect = msg_send![v, convertRect: vb, toView: null];
  let q: NSRect = msg_send![parent, convertRect: pb, toView: null];
  let flipped: Bool = msg_send![parent, isFlipped];
  let win: Obj = msg_send![v, window];
  let scale: f64 = msg_send![win, backingScaleFactor];
  println!("[{label}] scale={scale} parent_flipped={} parent={}x{} actual_top_left=({},{}) {}x{} bounds()={:?}",
    flipped.as_bool(), q.size.width, q.size.height,
    c.origin.x - q.origin.x, (q.origin.y + q.size.height) - (c.origin.y + c.size.height),
    c.size.width, c.size.height, wv.bounds().unwrap());
}
unsafe fn child(name: &str, parent: Obj) {
  let wv = WebViewBuilder::new().with_bounds(l(20, 20, 300, 200)).with_html("x").build_as_child(&H(parent)).unwrap();
  show(&format!("{name}: with_bounds logical(20,20,300,200)"), &wv);
  wv.set_bounds(l(100, 80, 200, 150)).unwrap();
  show(&format!("{name}: set logical(100,80,200,150)"), &wv);
  wv.set_bounds(p(100, 80, 200, 150)).unwrap();
  show(&format!("{name}: set physical(100,80,200,150)"), &wv);
}
unsafe fn view(cls: &AnyClass, host: Obj) -> Obj {
  let frame = NSRect::new(NSPoint::new(30., 40.), NSSize::new(500., 350.));
  let v: Obj = msg_send![cls, alloc];
  let v: Obj = msg_send![v, initWithFrame: frame];
  let _: () = msg_send![host, addSubview: v];
  v
}
fn main() {
  let el = EventLoop::new();
  let a = WindowBuilder::new().with_inner_size(LogicalSize::new(700., 500.)).build(&el).unwrap();
  let b = WindowBuilder::new().with_inner_size(LogicalSize::new(700., 500.)).build(&el).unwrap();
  el.run(move |ev, _, cf| {
    *cf = ControlFlow::Wait;
    if !matches!(ev, Event::NewEvents(StartCause::Init)) { return; }
    unsafe {
      let mut cb = ClassBuilder::new(c"VerifyFlipped", class!(NSView)).unwrap();
      cb.add_method(sel!(isFlipped), yes as extern "C-unwind" fn(_, _) -> _);
      let flipped = cb.register();
      let host = a.ns_view() as Obj;
      let full = WebViewBuilder::new().with_html("x").build(&b).unwrap();
      for pass in ["real scale", "forced scale 2"] {
        println!("##### {pass}");
        child("tao content view", host);
        child("NSView at (30,40)", view(class!(NSView), host));
        child("flipped NSView at (30,40)", view(flipped, host));
        show("full-window build(): before", &full);
        full.set_bounds(l(100, 80, 200, 150)).unwrap();
        show("full-window build(): after set logical(100,80,200,150)", &full);
        let m = class!(NSWindow).instance_method(sel!(backingScaleFactor)).unwrap();
        m.set_implementation(std::mem::transmute::<extern "C-unwind" fn(*mut AnyObject, Sel) -> f64, Imp>(two));
      }
    }
    println!("DONE");
    std::process::exit(0);
  });
}
