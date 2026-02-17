use core::mem::MaybeUninit;
use flint_sys;
//use gmp_mpfr_sys::{gmp, gmp::mpf_t, gmp::mpz_t};
use flint_sys::fmpq;
use flint_sys::fmpq::fmpq as mpf_t;
use flint_sys::fmpz;
use flint_sys::fmpz::fmpz as mpz_t;
use std::sync::Arc;
use std::thread;
use std::{env, ffi::CStr};
use tokio::sync::Mutex;

const A: u64 = 13591409;
const B: u64 = 545140134;
const C: u64 = 640320;
const D: u64 = 426880;
const E: u64 = 10005;
const C3_24: u64 = C.pow(3) / 24;
const THRESH: u64 = 10u64.pow(4) * 5;

struct PQT {
    p: mpz_t,
    q: mpz_t,
    t: mpz_t,
}

struct WrappedMpz {
    a: mpz_t,
}

struct WrappedMpf {
    a: mpf_t,
}

struct WrappedMpzBi {
    a: mpz_t,
    b: mpz_t,
}

struct WrappedMpzTri {
    a: mpz_t,
    b: mpz_t,
    c: mpz_t,
}

struct WrappedMpfTri {
    a: mpf_t,
    b: mpf_t,
    c: mpf_t,
}

unsafe impl Send for PQT {} // luckily, im not accessing anything between threads
impl Default for WrappedMpz {
    fn default() -> Self {
        WrappedMpz { a: allocate_mpz(0) }
    }
}

unsafe impl Send for WrappedMpz {}
unsafe impl Sync for WrappedMpz {}
unsafe impl Send for WrappedMpf {}
unsafe impl Send for WrappedMpzBi {}
unsafe impl Send for WrappedMpzTri {}
unsafe impl Send for WrappedMpfTri {}

fn allocate_mpz(init_value: u64) -> mpz_t {
    unsafe {
        let mut z = MaybeUninit::uninit();
        fmpz::fmpz_init(z.as_mut_ptr());
        let mut z = z.assume_init();
        fmpz::fmpz_set_ui(&mut z, init_value);
        z
    }
}

//fn allocate_fmpz(init_value: u64) -> fmpz::fmpz {
//    unsafe {
//        let mut z = MaybeUninit::uninit();
//        fmpz::fmpz_init(z.as_mut_ptr());
//        let mut z = z.assume_init();
//        fmpz::fmpz_set_ui(&mut z, init_value);
//        z
//    }
//}

unsafe fn allocate_mpf(init_value: u64, prec: u64) -> mpf_t {
    unsafe {
        let mut z = MaybeUninit::uninit();
        fmpq::fmpq_init(z.as_mut_ptr());
        let mut z = z.assume_init();
        fmpq::fmpq_set_ui_den1(&mut z, init_value);
        z
    }
}

fn make_cstr_mpf(fmt_str: mpf_t, digits: usize) -> String {
    let mut expptr: i64 = 0;
    unsafe {
        CStr::from_ptr(fmpq::fmpq_get_str(
            std::ptr::null_mut(),
            10i32,
            &fmt_str as *const mpf_t,
        ))
        .to_str()
        .unwrap()
        .to_string()
    }
}

fn make_cstr_mpz(fmt_str: mpz_t) -> String {
    unsafe {
        CStr::from_ptr(fmpz::fmpz_get_str(
            std::ptr::null_mut(),
            10i32,
            &fmt_str as *const mpz_t,
        ))
        .to_str()
        .unwrap()
        .to_string()
    }
}

// async fn calc_sqrt(prec: u64) -> WrappedMpf {
//     unsafe {
//         let mut e = allocate_mpf(E, prec);
//         fmpq::fmpq_sqrt(&mut e as *mut mpf_t, &e as *const mpf_t);
//         println!("e done");
//         WrappedMpf { a: e }
//     }
// }

async fn calc_sqrt_pell(prec: u64) -> WrappedMpzBi {
    unsafe {
        let mut xy = WrappedMpzBi {
            a: allocate_mpz(0),
            b: allocate_mpz(0),
        };
        let mut p1 = WrappedMpzBi {
            a: allocate_mpz(1u64),
            b: allocate_mpz(0u64),
        };
        let mut p2 = WrappedMpzBi {
            a: allocate_mpz(4001u64),
            b: allocate_mpz(40u64),
        };
        let mut tmp = WrappedMpz { a: allocate_mpz(0) };
        let mut target = WrappedMpz {
            a: allocate_mpz(10),
        };
        fmpz::fmpz_pow_ui(
            &mut target.a as *mut mpz_t,
            &target.a as *const mpz_t,
            (prec / 2) + 5,
        );
        loop {
            // x = x1*x2 + D*y1*y2
            let x1_wrap = WrappedMpz { a: p1.a };
            let x2_wrap = WrappedMpz { a: p2.a };
            let x_1_c_handle = tokio::spawn(async {
                let mut tmp = WrappedMpz { a: allocate_mpz(0) };
                let x1 = x1_wrap;
                let x2 = x2_wrap;
                fmpz::fmpz_mul(
                    &mut tmp.a as *mut mpz_t,
                    &x1.a as *const mpz_t,
                    &x2.a as *const mpz_t,
                );
                WrappedMpz { a: tmp.a }
            });
            let y1_wrap = WrappedMpz { a: p1.b };
            let y2_wrap = WrappedMpz { a: p2.b };
            let x_2_c_handle = tokio::spawn(async {
                let mut tmp = WrappedMpz { a: allocate_mpz(0) };
                let y1 = y1_wrap;
                let y2 = y2_wrap;
                fmpz::fmpz_mul(
                    &mut tmp.a as *mut mpz_t,
                    &y1.a as *const mpz_t,
                    &y2.a as *const mpz_t,
                );
                fmpz::fmpz_mul_ui(&mut tmp.a as *mut mpz_t, &tmp.a as *const mpz_t, E);
                WrappedMpz { a: tmp.a }
            });

            // y = x1*y2 + y1*x2
            let x1_wrap = WrappedMpz { a: p1.a };
            let y2_wrap = WrappedMpz { a: p2.b };
            let y_1_c_handle = tokio::spawn(async {
                let mut tmp = WrappedMpz { a: allocate_mpz(0) };
                let x1 = x1_wrap;
                let y2 = y2_wrap;
                fmpz::fmpz_mul(
                    &mut tmp.a as *mut mpz_t,
                    &x1.a as *const mpz_t,
                    &y2.a as *const mpz_t,
                );
                WrappedMpz { a: tmp.a }
            });
            let y1_wrap = WrappedMpz { a: p1.b };
            let x2_wrap = WrappedMpz { a: p2.a };
            let y_2_c_handle = tokio::spawn(async {
                let mut tmp = WrappedMpz { a: allocate_mpz(0) };
                let y1 = y1_wrap;
                let x2 = x2_wrap;
                fmpz::fmpz_mul(
                    &mut tmp.a as *mut mpz_t,
                    &y1.a as *const mpz_t,
                    &x2.a as *const mpz_t,
                );
                WrappedMpz { a: tmp.a }
            });
            //(x, y) = {
            let x_handle = tokio::spawn(async move {
                //let x_1_c = x_1_c_handle.await.unwrap();
                //let x_2_c = x_2_c_handle.await.unwrap();
                let (x_1_c, x_2_c) = tokio::join!(x_1_c_handle, x_2_c_handle);
                let tmp = WrappedMpz { a: allocate_mpz(0) };
                //fmpz::fmpz_add(
                //    &mut tmp as *mut mpz_t,
                //    &x_1_c.a as *const mpz_t,
                //    &x_2_c.a as *const mpz_t,
                //);
                mpz_add_ns(WrappedMpz { a: tmp.a }, x_1_c.unwrap(), x_2_c.unwrap())
                //WrappedMpz { a: tmp }
            });
            let y_handle = tokio::spawn(async move {
                //let y_1_c = y_1_c_handle.await.unwrap();
                //let y_2_c = y_2_c_handle.await.unwrap();
                let (y_1_c, y_2_c) = tokio::join!(y_1_c_handle, y_2_c_handle);
                let tmp = WrappedMpz { a: allocate_mpz(0) };
                //fmpz::fmpz_add(
                //    &mut tmp as *mut mpz_t,
                //    &y_1_c.a as *const mpz_t,
                //    &y_2_c.a as *const mpz_t,
                //);
                mpz_add_ns(WrappedMpz { a: tmp.a }, y_1_c.unwrap(), y_2_c.unwrap())
                //WrappedMpz { a: tmp }
            });
            // let y_h = y_handle.await;
            //let x_r = x_handle.await.unwrap().a;
            //let y_r = y_handle.await.unwrap().a;
            let (x_r, y_r) = tokio::join!(x_handle, y_handle);
            fmpz::fmpz_set(&mut xy.a as *mut mpz_t, &x_r.unwrap().a as *const mpz_t);
            fmpz::fmpz_set(&mut xy.b as *mut mpz_t, &y_r.unwrap().a as *const mpz_t);
            //(x, y)
            //};
            if fmpz::fmpz_cmp(&xy.b as *const mpz_t, &target.a as *const mpz_t) > 0 {
                // should dealloc memory here
                break;
            }
            fmpz::fmpz_set(&mut p1.a as *mut mpz_t, &p2.a as *const mpz_t);
            fmpz::fmpz_set(&mut p1.b as *mut mpz_t, &p2.b as *const mpz_t);
            fmpz::fmpz_set(&mut p2.a as *mut mpz_t, &xy.a as *const mpz_t);
            fmpz::fmpz_set(&mut p2.b as *mut mpz_t, &xy.b as *const mpz_t);
        }
        println!("e done");
        WrappedMpzBi { a: xy.a, b: xy.b }
    }
}

async fn mpf_mul(mut wrap: WrappedMpfTri) -> WrappedMpf {
    unsafe {
        fmpq::fmpq_mul(
            &mut wrap.a as *mut mpf_t,
            &wrap.b as *const mpf_t,
            &wrap.c as *const mpf_t,
        );
        WrappedMpf { a: wrap.a }
    }
}

async fn mpf_add(mut wrap: WrappedMpfTri) -> WrappedMpf {
    unsafe {
        fmpq::fmpq_add(
            &mut wrap.a as *mut mpf_t,
            &wrap.b as *const mpf_t,
            &wrap.c as *const mpf_t,
        );
        WrappedMpf { a: wrap.a }
    }
}

fn mpz_add_ns(mut a: WrappedMpz, b: WrappedMpz, c: WrappedMpz) -> WrappedMpz {
    unsafe {
        fmpz::fmpz_add(
            &mut a.a as *mut mpz_t,
            &b.a as *const mpz_t,
            &c.a as *const mpz_t,
        );
        WrappedMpz { a: a.a }
    }
}

async fn mpf_cast(wrap: WrappedMpz, prec: u64) -> WrappedMpf {
    unsafe {
        let mut cast = allocate_mpf(0, prec);
        fmpq::fmpq_set_fmpz_den1(&mut cast as *mut mpf_t, &wrap.a as *const mpz_t);
        WrappedMpf { a: cast }
    }
}

fn i_compute_pqt(n1: u64, n2: u64) -> PQT {
    // async unsafe fn compute_pqt(a: mpz_t, b: mpz_t, c3_24: mpz_t, n1: u64, n2: u64) -> PQT {
    // println!("compute_pqt called {} {}", n1, n2);
    let mut pqt: PQT = PQT {
        p: allocate_mpz(0),
        q: allocate_mpz(0),
        t: allocate_mpz(0),
    };
    unsafe {
        if n1 + 1 == n2 {
            let p_mut = &mut pqt.p as *mut mpz_t;
            let p_const = &pqt.p as *const mpz_t;
            fmpz::fmpz_set_ui(p_mut, 2 * n2 - 1);
            fmpz::fmpz_mul_ui(p_mut, p_const, 6 * n2 - 1);
            fmpz::fmpz_mul_ui(p_mut, p_const, 6 * n2 - 5);
            let q_mut = &mut pqt.q as *mut mpz_t;
            let q_const = &pqt.q as *const mpz_t;
            fmpz::fmpz_set_ui(q_mut, C3_24);
            let mut n2_3 = allocate_mpz(n2);
            fmpz::fmpz_set_ui(&mut pqt.t as *mut mpz_t, A);
            fmpz::fmpz_addmul_ui(&mut pqt.t as *mut mpz_t, &n2_3 as *const mpz_t, B);
            fmpz::fmpz_mul(
                &mut pqt.t as *mut mpz_t,
                &pqt.t as *const mpz_t,
                &pqt.p as *const mpz_t,
            );
            if (n2 & 1) == 1 {
                fmpz::fmpz_neg(&mut pqt.t as *mut mpz_t, &pqt.t as *const mpz_t);
            }
            fmpz::fmpz_pow_ui(
                &mut n2_3 as *mut mpz_t,
                &allocate_mpz(n2) as *const mpz_t,
                3,
            );
            fmpz::fmpz_mul(q_mut, q_const, &n2_3 as *const mpz_t);
            fmpz::fmpz_clear(&mut n2_3);
        } else {
            let m = (n1 + n2) / 2;
            // single thread
            let mut res1 = i_compute_pqt(n1, m); // res1 is used as a temp buffer to reduce mem
            let mut res2 = i_compute_pqt(m, n2);
            fmpz::fmpz_mul(
                &mut pqt.p as *mut mpz_t,
                &res1.p as *const mpz_t,
                &res2.p as *const mpz_t,
            );
            fmpz::fmpz_mul(
                &mut pqt.q as *mut mpz_t,
                &res1.q as *const mpz_t,
                &res2.q as *const mpz_t,
            );
            let mut t_1 = allocate_mpz(0);
            let mut t_2 = allocate_mpz(0);
            fmpz::fmpz_mul(
                &mut t_1 as *mut mpz_t,
                &res1.t as *const mpz_t,
                &res2.q as *const mpz_t,
            );
            fmpz::fmpz_mul(
                &mut t_2 as *mut mpz_t,
                &res1.p as *const mpz_t,
                &res2.t as *const mpz_t,
            );
            fmpz::fmpz_add(
                &mut pqt.t as *mut mpz_t,
                &t_1 as *const mpz_t,
                &t_2 as *const mpz_t,
            );

            fmpz::fmpz_clear(&mut res1.p);
            fmpz::fmpz_clear(&mut res1.q);
            fmpz::fmpz_clear(&mut res1.t);
            fmpz::fmpz_clear(&mut res2.p);
            fmpz::fmpz_clear(&mut res2.q);
            fmpz::fmpz_clear(&mut res2.t);
            fmpz::fmpz_clear(&mut t_1);
            fmpz::fmpz_clear(&mut t_2);
        }
    }
    pqt
}

#[async_recursion::async_recursion]
async fn compute_pqt(n1: u64, n2: u64) -> PQT {
    // async unsafe fn compute_pqt(a: mpz_t, b: mpz_t, c3_24: mpz_t, n1: u64, n2: u64) -> PQT {
    // println!("compute_pqt called {} {}", n1, n2);
    if n1 + 1 == n2 {
        return i_compute_pqt(n1, n2);
    }
    let mut pqt: PQT = PQT {
        p: allocate_mpz(0),
        q: allocate_mpz(0),
        t: allocate_mpz(0),
    };
    let t_1 = WrappedMpz { a: allocate_mpz(0) };
    let t_2 = WrappedMpz { a: allocate_mpz(0) };
    unsafe {
        let m = (n1 + n2) / 2;
        let mut res1: PQT;
        let mut res2: PQT;
        if n2 - n1 < THRESH {
            res1 = i_compute_pqt(n1, m);
            res2 = i_compute_pqt(m, n2);
        } else {
            // single thread
            //let mut res1 = compute_pqt(n1, m).await; // res1 is used as a temp buffer to reduce mem
            //let mut res2 = compute_pqt(m, n2).await;
            // multi thread
            let res1_hook = tokio::spawn(compute_pqt(n1, m));
            let res2_hook = tokio::spawn(compute_pqt(m, n2));
            res1 = res1_hook.await.unwrap();
            res2 = res2_hook.await.unwrap();
        }
        if n2 - n1 > THRESH {
            println!("{}", n2 - n1);
        }
        // p = res1 p * res2 p
        let wrap_p = WrappedMpzTri {
            a: pqt.p,
            b: res1.p,
            c: res2.p,
        };
        //let p_thread = tokio::task::spawn_blocking(move || wrap_mul(wrap_p));
        let p_thread = tokio::spawn(async move {
            let mut wrap = wrap_p;
            fmpz::fmpz_mul(
                &mut wrap.a as *mut mpz_t,
                &wrap.b as *const mpz_t,
                &wrap.c as *const mpz_t,
            );
            WrappedMpz { a: wrap.a }
        });
        // let p_thread = tokio::spawn(move || wrap_mul(wrap_p));
        //let p_thread = tokio::spawn(move || {
        //    wrap_mul(wrap_p);
        //});
        // q = res1 q * res2 q
        let wrap_q = WrappedMpzTri {
            a: pqt.q,
            b: res1.q,
            c: res2.q,
        };
        let q_thread = tokio::spawn(async move {
            let mut wrap = wrap_q;
            fmpz::fmpz_mul(
                &mut wrap.a as *mut mpz_t,
                &wrap.b as *const mpz_t,
                &wrap.c as *const mpz_t,
            );
            WrappedMpz { a: wrap.a }
        });
        // let q_thread = tokio::spawn(move || wrap_mul(wrap_q));
        //let q_thread = tokio::spawn(move || {
        //    wrap_mul(wrap_q);
        //});
        //let mut t_1 = WrappedMpz::default();
        //let mut t_2 = WrappedMpz::default();

        //let t_1 = allocate_mpz(0);
        let t_1_wrap = WrappedMpzTri {
            a: t_1.a,
            b: res1.t,
            c: res2.q,
        };
        let t_1_handle = tokio::spawn(async move {
            let mut wrap = t_1_wrap;
            fmpz::fmpz_mul(
                &mut wrap.a as *mut mpz_t,
                &wrap.b as *const mpz_t,
                &wrap.c as *const mpz_t,
            );
            WrappedMpz { a: wrap.a }
        });
        // let t_1_thread = tokio::spawn(move || wrap_mul(wrap_t_1));
        //let t_1_thread = tokio::spawn(move || {
        //    wrap_mul(t_1_wrap);
        //});

        //let t_2 = allocate_mpz(0);
        let t_2_wrap = WrappedMpzTri {
            a: t_2.a,
            b: res1.p,
            c: res2.t,
        };
        let t_2_handle = tokio::spawn(async move {
            let mut wrap = t_2_wrap;
            fmpz::fmpz_mul(
                &mut wrap.a as *mut mpz_t,
                &wrap.b as *const mpz_t,
                &wrap.c as *const mpz_t,
            );
            WrappedMpz { a: wrap.a }
        });
        // let t_2_thread = tokio::spawn(move || wrap_mul(wrap_t_2));
        //let t_2_thread = tokio::spawn(move || {
        //    wrap_mul(t_2_wrap);
        //});
        let mut t_1 = t_1_handle.await.unwrap();
        let mut t_2 = t_2_handle.await.unwrap();
        fmpz::fmpz_add(
            &mut pqt.t as *mut mpz_t,
            &t_1.a as *const mpz_t,
            &t_2.a as *const mpz_t,
        );
        pqt.p = p_thread.await.unwrap().a;
        pqt.q = q_thread.await.unwrap().a;
        fmpz::fmpz_clear(&mut res1.p);
        fmpz::fmpz_clear(&mut res1.q);
        fmpz::fmpz_clear(&mut res1.t);
        fmpz::fmpz_clear(&mut res2.p);
        fmpz::fmpz_clear(&mut res2.q);
        fmpz::fmpz_clear(&mut res2.t);
        fmpz::fmpz_clear(&mut t_1.a);
        fmpz::fmpz_clear(&mut t_2.a);
    }
    pqt
}

#[tokio::main(flavor = "multi_thread", worker_threads = 50)]
async fn main() {
    let digits = env::args().nth(1).unwrap().parse::<u32>().unwrap();
    println!("Computing {} digits", digits);
    let prec = (digits * 10u32.ilog2()) as u64;
    // let comp: Chudnovsky = Chudnovsky::default();
    let digits_per_term = (53360f64.powf(3f64).ln()) / 10f64.ln();
    let n = digits as f64 / digits_per_term;
    unsafe {
        let e_handle = tokio::spawn(async move { calc_sqrt_pell(digits as u64).await });
        // let e_handle = thread::spawn(async move || calc_sqrt_pell(digits as u64));
        let mut c3_24 = allocate_mpz(C);
        fmpz::fmpz_pow_ui(&mut c3_24 as *mut mpz_t, &c3_24 as *const mpz_t, 3);
        fmpz::fmpz_divexact_ui(&mut c3_24 as *mut mpz_t, &c3_24 as *const mpz_t, 24);
        let pqt: PQT = compute_pqt(0u64, n as u64).await;
        // let pqt: PQT = i_compute_pqt(0u64, n as u64);
        println!("pqt done");
        let wrap_q = WrappedMpz { a: pqt.q };
        let wrap_t = WrappedMpz { a: pqt.t };
        let q_handle = tokio::spawn(async move {
            let q = mpf_cast(wrap_q, prec).await.a;
            let wrap_q_mpf = WrappedMpf { a: q };
            let mut q_clone = allocate_mpf(0, prec);
            fmpq::fmpq_set(&mut q_clone, &q as *const mpf_t);
            let wrap_q_mpf_clone = WrappedMpf { a: q_clone };
            let d_handle = tokio::spawn(async move {
                let mut d = allocate_mpf(D, prec);
                let q = wrap_q_mpf;
                let wrap = WrappedMpfTri { a: d, b: d, c: q.a };
                d = mpf_mul(wrap).await.a;
                // println!("d done");
                WrappedMpf { a: d }
            });
            let a_handle = tokio::spawn(async move {
                let mut a = allocate_mpf(A, prec);
                let q = wrap_q_mpf_clone;
                let wrap = WrappedMpfTri { a: a, b: a, c: q.a };
                a = mpf_mul(wrap).await.a;
                println!("a done");
                WrappedMpf { a: a }
            });
            (d_handle, a_handle)
        });
        let t_handle = tokio::spawn(async move { mpf_cast(wrap_t, prec).await });
        let (d_handle, a_handle) = q_handle.await.unwrap();
        // let top_handle = tokio::spawn(async {
        //     let e_grip = e_handle.await.unwrap();
        //     let d_grip = d_handle.await.unwrap();
        //     let mut e = e_grip.await.a;
        //     let d = d_grip.a;
        //     fmpq::fmpq_mul(&mut e as *mut mpf_t, &e as *const mpf_t, &d as *const mpf_t);
        //     // println!("top done");
        //     WrappedMpf { a: e }
        // });
        let bottom_handle = tokio::spawn(async {
            let a_grip = a_handle.await.unwrap();
            let t_grip = t_handle.await.unwrap();
            let mut a = a_grip.a;
            let t = t_grip;
            // println!("a t {} {}", make_cstr_mpf(a, 100), make_cstr_mpf(t.a, 100));
            fmpq::fmpq_add(
                &mut a as *mut mpf_t,
                &a as *const mpf_t,
                &t.a as *const mpf_t,
            );
            // println!("bottom done");
            WrappedMpf { a: a }
        });
        // let e_comp = e_handle.join().unwrap().await.await;
        let e_comp = e_handle.await.unwrap();
        let e_f = e_comp;
        let e_x = WrappedMpz { a: e_f.a };
        let e_y = WrappedMpz { a: e_f.b };
        // println!("{} {}", make_cstr_mpz(e_x.a), make_cstr_mpz(e_y.a));
        let top_handle = tokio::spawn(async {
            let e_x_mpf = mpf_cast(e_x, 0u64).await;
            let d_grip = d_handle.await.unwrap();
            let mut d = d_grip.a;
            fmpq::fmpq_mul(
                &mut d as *mut mpf_t,
                &e_x_mpf.a as *const mpf_t,
                &d as *const mpf_t,
            );
            // println!("top done");
            WrappedMpf { a: d }
        });
        let mut pi = allocate_mpf(0, prec);
        let bottom_mul_handle = tokio::spawn(async {
            let e_y_mpf = mpf_cast(e_y, 0u64).await;
            let mut bottom = bottom_handle.await.unwrap().a;
            fmpq::fmpq_mul(
                &mut bottom as *mut mpf_t,
                &bottom as *const mpf_t,
                &e_y_mpf.a as *const mpf_t,
            );
            WrappedMpf { a: bottom }
        });
        let top = top_handle.await.unwrap().a;
        let bottom = bottom_mul_handle.await.unwrap().a;
        // println!("meow {}", make_cstr_mpf(bottom, 100));
        fmpq::fmpq_div(
            &mut pi as *mut mpf_t,
            &top as *const mpf_t,
            &bottom as *const mpf_t,
        );
        //println!("casts done");
        //fmpq::fmpq_sqrt(&mut e as *mut mpf_t, &e as *const mpf_t);
        //println!("sqrt done");
        //fmpq::fmpq_mul(&mut e as *mut mpf_t, &e as *const mpf_t, &q as *const mpf_t);
        // let d_hook = tokio::spawn(async { mpf_mul(WrappedMpfTri { a: d, b: d, c: q }) });
        // fmpq::fmpq_mul(
        //     &mut d as *mut mpf_t,
        //     &d as *const mpf_t,
        //     &pqt.t as *const mpf_t,
        // );
        // println!("mul1 done");
        // fmpq::fmpq_mul(
        //     &mut pi as *mut mpf_t,
        //     &e as *const mpf_t,
        //     &allocate_mpf(D, prec) as *const mpf_t, // d
        // );
        // println!("mul2 done");
        // // e is not representative here. just saving mem
        // fmpq::fmpq_mul(
        //     &mut e as *mut mpf_t,
        //     &q as *const mpf_t,
        //     &allocate_mpf(A, prec) as *const mpf_t,
        // );
        // fmpq::fmpq_add(&mut e as *mut mpf_t, &e as *const mpf_t, &t as *const mpf_t);
        // fmpq::fmpq_div(
        //     &mut pi as *mut mpf_t,
        //     &pi as *const mpf_t,
        //     &e as *const mpf_t,
        // );
        println!("computed, making string");
        let printout = make_cstr_mpf(pi, digits as usize);
        println!("{printout}");
    }
}
