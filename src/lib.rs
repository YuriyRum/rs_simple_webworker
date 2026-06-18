use js_sys::Function;
use js_sys::Promise;
use std::cell::RefCell;
use std::rc::Rc;
use wasm_bindgen::closure::Closure;
use wasm_bindgen::prelude::*;
use web_sys::Blob;
use web_sys::Url;

#[wasm_bindgen]
struct Worker {
    // callback: Option<Closure<dyn FnMut(web_sys::MessageEvent)>>,
    webworker: web_sys::Worker,
    future: Promise,
    #[warn(dead_code)]
    onmessage_closure: Option<Closure<dyn FnMut(web_sys::MessageEvent)>>,
}

#[wasm_bindgen]
impl Worker {
    fn new() -> Self {
        let template = "    
            onmessage = async (e) => {
                // if(!self[e.data.name]) {
                    importScripts(e.data.url); 
                // } 
                if(Array.isArray(e.data.deps) && e.data.deps.length > 0) {
                    importScripts(...e.data.deps);
                }
                
                if(!self.globalState && e.data.globalState) {
                    self.globalState = e.data.globalState;
                }  

                if(self[e.data.name]) {
                    const result = await self[e.data.name](...e.data.args);
                    self.postMessage(result);
                } else {
                    console.error(`Worker does not have function ${e.data.name}`);
                }
            }; ";
        let blob_parts = js_sys::Array::new();
        blob_parts.push(&JsValue::from_str(template));
        let blob = Blob::new_with_str_sequence(&blob_parts).unwrap();
        let url = Url::create_object_url_with_blob(&blob).unwrap();
        let webworker = web_sys::Worker::new(&url).unwrap();

        let resolver: Rc<RefCell<Option<Function>>> = Rc::new(RefCell::new(None));

        let promise = Promise::new(&mut |resolve, _reject| {
            *resolver.borrow_mut() = Some(resolve);
        });

        let resolver_for_clouser = resolver.clone();
        let onmessage_closure = Closure::wrap(Box::new(move |e: web_sys::MessageEvent| {
            if let Some(resolve) = resolver_for_clouser.borrow_mut().take() {
                resolve.call1(&JsValue::NULL, &e.data()).unwrap();
            }
        }) as Box<dyn FnMut(_)>);

        // webworker.set_onmessage(Some(onmessage_closure.as_ref().unchecked_ref()));
        webworker
            .add_event_listener_with_callback("message", onmessage_closure.as_ref().unchecked_ref())
            .unwrap();
        // onmessage_closure.forget();
        Worker {
            webworker,
            future: promise,
            onmessage_closure: Some(onmessage_closure),
        }
    }

    fn terminate(&self) {
        self.webworker
            .remove_event_listener_with_callback(
                "message",
                self.onmessage_closure
                    .as_ref()
                    .unwrap()
                    .as_ref()
                    .unchecked_ref(),
            )
            .unwrap();
        self.webworker.terminate();
    }

    fn refresh_onmessage(&mut self) {
        let resolver: Rc<RefCell<Option<Function>>> = Rc::new(RefCell::new(None));
        let promise = Promise::new(&mut |resolve, _reject| {
            *resolver.borrow_mut() = Some(resolve);
        });
        let resolver_for_clouser = resolver.clone();
        let onmessage_closure = Closure::wrap(Box::new(move |e: web_sys::MessageEvent| {
            if let Some(resolve) = resolver_for_clouser.borrow_mut().take() {
                resolve.call1(&JsValue::NULL, &e.data()).unwrap();
            }
        }) as Box<dyn FnMut(_)>);

        self.webworker
            .remove_event_listener_with_callback(
                "message",
                self.onmessage_closure
                    .as_ref()
                    .unwrap()
                    .as_ref()
                    .unchecked_ref(),
            )
            .unwrap();
        // self.webworker
        //     .set_onmessage(Some(onmessage_closure.as_ref().unchecked_ref()));
        self.webworker
            .add_event_listener_with_callback("message", onmessage_closure.as_ref().unchecked_ref())
            .unwrap();
        self.onmessage_closure = Some(onmessage_closure);
        self.future = promise;
    }

    fn exec(
        &self,
        fun_name: &str,
        func: js_sys::Function,
        args: Vec<wasm_bindgen::JsValue>,
        deps: Vec<String>,
        global_state: Option<JsValue>,
    ) -> Promise {
        let url = Url::create_object_url_with_blob(
            &Blob::new_with_str_sequence(&js_sys::Array::of1(&func)).unwrap(),
        )
        .unwrap();
        let message = js_sys::Object::new();
        js_sys::Reflect::set(
            &message,
            &JsValue::from_str("url"),
            &JsValue::from_str(&url),
        )
        .unwrap();
        js_sys::Reflect::set(
            &message,
            &JsValue::from_str("name"),
            &JsValue::from_str(fun_name),
        )
        .unwrap();

        let js_args = js_sys::Array::new();
        for arg in args {
            js_args.push(&arg);
        }
        js_sys::Reflect::set(&message, &JsValue::from_str("args"), &js_args.into()).unwrap();

        let js_deps = js_sys::Array::new();
        for dep in deps {
            js_deps.push(&JsValue::from_str(&dep));
        }
        js_sys::Reflect::set(&message, &JsValue::from_str("deps"), &js_deps.into()).unwrap();

        if let Some(state) = global_state {
            js_sys::Reflect::set(&message, &JsValue::from_str("globalState"), &state).unwrap();
        }
        self.webworker.post_message(&message).unwrap();
        self.future.clone()
    }
}

/// Struct represents a pool of web workers that can execute functions in parallel.
#[wasm_bindgen]
pub struct WorkerPool {
    workers: Vec<Option<Worker>>,
}

#[wasm_bindgen]
impl WorkerPool {
    /// Creates a new worker pool with the specified number of workers.
    pub fn new(number_of_workers: usize) -> Self {
        console_error_panic_hook::set_once();
        let mut workers = Vec::with_capacity(number_of_workers);
        for _ in 0..number_of_workers {
            workers.push(None);
        }
        WorkerPool { workers }
    }
    /// Destroys the worker at the specified index, if it exists.
    pub fn destroy_worker(&mut self, index: usize) {
        if let Some(Some(worker)) = self.workers.get(index) {
            worker.terminate();
            self.workers[index] = None;
        }
    }
    /// Destroys all workers in the pool.
    pub fn destroy_all_workers(&mut self) {
        for worker_option in self.workers.iter_mut() {
            if let Some(worker) = worker_option {
                worker.terminate();
            }
            *worker_option = None;
        }
    }
    /// Executes the specified function with the given function name, Vec of arguments, Vec of dependencies and global state on the worker at the specified index.
    /// If the worker does not exist, it will be created. If the worker already exists, it will be reused.
    /// Global state is an optional parameter that allows you to set a global state for the worker, that can be reused between executions for different functions. It will be available as `globalThis.globalState` in the worker's context.
    pub fn get_worker_and_execute(
        &mut self,
        index: usize,
        fun_name: &str,
        func: js_sys::Function,
        args: Vec<wasm_bindgen::JsValue>,
        deps: Vec<String>,
        global_state: Option<JsValue>,
    ) -> Promise {
        let Some(worker_option) = self.workers.get_mut(index) else {
            return Promise::reject(&JsValue::from_str("Worker index out of bounds"));
        };
        let Some(worker) = worker_option.as_mut() else {
            let worker = Worker::new();
            self.workers.insert(index, Some(worker));
            return self.workers.get(index).unwrap().as_ref().unwrap().exec(
                fun_name,
                func,
                args,
                deps,
                global_state,
            );
        };
        worker.refresh_onmessage();
        worker.exec(fun_name, func, args, deps, global_state)
    }

    /// Returns actual size of the pool
    pub fn get_pool_size(&self) -> usize {
        self.workers.len()
    }

    /// Returns the number of active workers in the pool.
    pub fn get_number_of_active_workers(&self) -> usize {
        self.workers.iter().filter(|w| w.is_some()).count()
    }

    /// Returns the index of the first free worker in the pool, or None if all workers are busy.
    pub fn get_free_worker_index(&self) -> Option<usize> {
        self.workers.iter().position(|w| w.is_none())
    }

    /// Returns the index of the first free worker in the pool, or creates a new worker if all workers are busy.
    pub fn get_or_add_free_worker_index(&mut self) -> usize {
        if let Some(index) = self.get_free_worker_index() {
            return index;
        }
        let new_index = self.workers.len();
        self.workers.push(None);
        new_index
    }
}
