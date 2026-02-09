use core::mem::MaybeUninit;
use gmp_mpfr_sys::{gmp, gmp::mpf_t, gmp::mpz_t};
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
const THRESH: u64 = 10u64.pow(5);

struct PQT {
    p: gmp::mpz_t,
    q: gmp::mpz_t,
    t: gmp::mpz_t,
}

struct WrappedMpz {
    a: gmp::mpz_t,
}

struct WrappedMpzTri {
    a: gmp::mpz_t,
    b: gmp::mpz_t,
    c: gmp::mpz_t,
}

unsafe impl Send for PQT {} // luckily, im not accessing anything between threads
impl Default for WrappedMpz {
    fn default() -> Self {
        WrappedMpz { a: allocate_mpz(0) }
    }
}
unsafe impl Send for WrappedMpz {}
unsafe impl Send for WrappedMpzTri {}

fn allocate_mpz(init_value: u64) -> gmp::mpz_t {
    unsafe {
        let mut z = MaybeUninit::uninit();
        gmp::mpz_init(z.as_mut_ptr());
        let mut z = z.assume_init();
        gmp::mpz_set_ui(&mut z, init_value);
        z
    }
}

unsafe fn allocate_mpf(init_value: u64, prec: u64) -> gmp::mpf_t {
    unsafe {
        let mut z = MaybeUninit::uninit();
        gmp::mpf_init(z.as_mut_ptr());
        let mut z = z.assume_init();
        gmp::mpf_set_ui(&mut z, init_value);
        gmp::mpf_set_prec(&mut z, prec);
        z
    }
}

fn make_cstr_mpf(fmt_str: mpf_t, digits: usize) -> String {
    let mut expptr: i64 = 0;
    unsafe {
        CStr::from_ptr(gmp::mpf_get_str(
            std::ptr::null_mut(),
            &mut expptr,
            10i32,
            digits,
            &fmt_str as *const mpf_t,
        ))
        .to_str()
        .unwrap()
        .to_string()
    }
}

fn make_cstr_mpz(fmt_str: mpz_t) -> String {
    unsafe {
        CStr::from_ptr(gmp::mpz_get_str(
            std::ptr::null_mut(),
            10i32,
            &fmt_str as *const mpz_t,
        ))
        .to_str()
        .unwrap()
        .to_string()
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
            gmp::mpz_set_ui(p_mut, 2 * n2 - 1);
            gmp::mpz_mul_ui(p_mut, p_const, 6 * n2 - 1);
            gmp::mpz_mul_ui(p_mut, p_const, 6 * n2 - 5);
            let q_mut = &mut pqt.q as *mut mpz_t;
            let q_const = &pqt.q as *const mpz_t;
            gmp::mpz_set_ui(q_mut, C3_24);
            let mut n2_3 = allocate_mpz(n2);
            gmp::mpz_set_ui(&mut pqt.t as *mut mpz_t, A);
            gmp::mpz_addmul_ui(&mut pqt.t as *mut mpz_t, &n2_3 as *const mpz_t, B);
            gmp::mpz_mul(
                &mut pqt.t as *mut mpz_t,
                &pqt.t as *const mpz_t,
                &pqt.p as *const mpz_t,
            );
            if (n2 & 1) == 1 {
                gmp::mpz_neg(&mut pqt.t as *mut mpz_t, &pqt.t as *const mpz_t);
            }
            gmp::mpz_ui_pow_ui(&mut n2_3 as *mut mpz_t, n2, 3);
            gmp::mpz_mul(q_mut, q_const, &n2_3 as *const mpz_t);
            gmp::mpz_clear(&mut n2_3);
        } else {
            let m = (n1 + n2) / 2;
            // single thread
            let mut res1 = i_compute_pqt(n1, m); // res1 is used as a temp buffer to reduce mem
            let mut res2 = i_compute_pqt(m, n2);
            gmp::mpz_mul(
                &mut pqt.p as *mut mpz_t,
                &res1.p as *const mpz_t,
                &res2.p as *const mpz_t,
            );
            gmp::mpz_mul(
                &mut pqt.q as *mut mpz_t,
                &res1.q as *const mpz_t,
                &res2.q as *const mpz_t,
            );
            let mut t_1 = allocate_mpz(0);
            let mut t_2 = allocate_mpz(0);
            gmp::mpz_mul(
                &mut t_1 as *mut mpz_t,
                &res1.t as *const mpz_t,
                &res2.q as *const mpz_t,
            );
            gmp::mpz_mul(
                &mut t_2 as *mut mpz_t,
                &res1.p as *const mpz_t,
                &res2.t as *const mpz_t,
            );
            gmp::mpz_add(
                &mut pqt.t as *mut mpz_t,
                &t_1 as *const mpz_t,
                &t_2 as *const mpz_t,
            );

            gmp::mpz_clear(&mut res1.p);
            gmp::mpz_clear(&mut res1.q);
            gmp::mpz_clear(&mut res1.t);
            gmp::mpz_clear(&mut res2.p);
            gmp::mpz_clear(&mut res2.q);
            gmp::mpz_clear(&mut res2.t);
            gmp::mpz_clear(&mut t_1);
            gmp::mpz_clear(&mut t_2);
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
    let mut t_1 = WrappedMpz { a: allocate_mpz(0) };
    let mut t_2 = WrappedMpz { a: allocate_mpz(0) };
    unsafe {
        let m = (n1 + n2) / 2;
        let mut res1: PQT;
        let mut res2: PQT;
        if n2 - n1 < THRESH {
            res1 = compute_pqt(n1, m).await;
            res2 = compute_pqt(m, n2).await;
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
        if n2 - n1 > 1000000 {
            println!("{}", n2);
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
            gmp::mpz_mul(
                &mut wrap.a as *mut mpz_t,
                &wrap.b as *const mpz_t,
                &wrap.c as *const mpz_t,
            );
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
            gmp::mpz_mul(
                &mut wrap.a as *mut mpz_t,
                &wrap.b as *const mpz_t,
                &wrap.c as *const mpz_t,
            );
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
            gmp::mpz_mul(
                &mut wrap.a as *mut mpz_t,
                &wrap.b as *const mpz_t,
                &wrap.c as *const mpz_t,
            );
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
            gmp::mpz_mul(
                &mut wrap.a as *mut mpz_t,
                &wrap.b as *const mpz_t,
                &wrap.c as *const mpz_t,
            );
        });
        // let t_2_thread = tokio::spawn(move || wrap_mul(wrap_t_2));
        //let t_2_thread = tokio::spawn(move || {
        //    wrap_mul(t_2_wrap);
        //});
        let _ = t_1_handle.await.unwrap();
        let _ = t_2_handle.await.unwrap();
        gmp::mpz_mul(
            &mut pqt.t as *mut mpz_t,
            &t_1.a as *const mpz_t,
            &t_2.a as *const mpz_t,
        );
        let _ = p_thread.await.unwrap();
        let _ = q_thread.await.unwrap();
        println!("2 {}", make_cstr_mpz(res1.p));
        gmp::mpz_clear(&mut res1.p);
        gmp::mpz_clear(&mut res1.q);
        gmp::mpz_clear(&mut res1.t);
        gmp::mpz_clear(&mut res2.p);
        gmp::mpz_clear(&mut res2.q);
        gmp::mpz_clear(&mut res2.t);
        gmp::mpz_clear(&mut t_1.a);
        gmp::mpz_clear(&mut t_2.a);
    }
    pqt
}

#[tokio::main(flavor = "multi_thread", worker_threads = 1)]
async fn main() {
    let digits = env::args().nth(1).unwrap().parse::<u32>().unwrap();
    println!("Computing {} digits", digits);
    let prec = (digits * 10u32.ilog2()) as u64;
    // let comp: Chudnovsky = Chudnovsky::default();
    let digits_per_term = (53360f64.powf(3f64).ln()) / 10f64.ln();
    let n = digits as f64 / digits_per_term;
    unsafe {
        let mut c3_24 = allocate_mpz(C);
        gmp::mpz_pow_ui(&mut c3_24 as *mut mpz_t, &c3_24 as *const mpz_t, 3);
        gmp::mpz_divexact_ui(&mut c3_24 as *mut mpz_t, &c3_24 as *const mpz_t, 24);
        //let pqt: PQT = compute_pqt(0u64, n as u64).await;
        let pqt: PQT = i_compute_pqt(0u64, n as u64);
        println!("pqt done");
        let mut pi = allocate_mpf(0, prec);
        let mut e = allocate_mpf(E, prec); // ei
        let mut q = allocate_mpf(0, prec);
        let mut t = allocate_mpf(0, prec);
        gmp::mpf_set_z(&mut q as *mut mpf_t, &pqt.q as *const mpz_t);
        gmp::mpf_set_z(&mut t as *mut mpf_t, &pqt.t as *const mpz_t);
        println!("casts done");
        gmp::mpf_sqrt(&mut e as *mut mpf_t, &e as *const mpf_t);
        println!("sqrt done");
        gmp::mpf_mul(&mut e as *mut mpf_t, &e as *const mpf_t, &q as *const mpf_t);
        println!("mul1 done");
        gmp::mpf_mul(
            &mut pi as *mut mpf_t,
            &e as *const mpf_t,
            &allocate_mpf(D, prec) as *const mpf_t, // d
        );
        println!("mul2 done");
        // e is not representative here. just saving mem
        gmp::mpf_mul(
            &mut e as *mut mpf_t,
            &q as *const mpf_t,
            &allocate_mpf(A, prec) as *const mpf_t,
        );
        gmp::mpf_add(&mut e as *mut mpf_t, &e as *const mpf_t, &t as *const mpf_t);
        gmp::mpf_div(
            &mut pi as *mut mpf_t,
            &pi as *const mpf_t,
            &e as *const mpf_t,
        );
        println!("computed, making string");
        let printout = make_cstr_mpf(pi, digits as usize);
        println!("{printout}");
    }
}
