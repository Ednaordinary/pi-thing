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

unsafe fn mpz_add(mut a: mpz_t, b: mpz_t, c: mpz_t) {
    unsafe { gmp::mpz_add(&mut a as *mut mpz_t, &b as *const mpz_t, &c as *const mpz_t) }
}

unsafe fn mpz_mul(mut a: mpz_t, b: mpz_t, c: mpz_t) {
    unsafe { gmp::mpz_mul(&mut a as *mut mpz_t, &b as *const mpz_t, &c as *const mpz_t) }
}

unsafe fn wrap_mul(wrap: WrappedMpzTri) {
    unsafe {
        mpz_mul(wrap.a, wrap.b, wrap.c);
    }
}

unsafe fn wrap_mul_ret(wrap: WrappedMpzTri) -> () {
    unsafe { wrap_mul(wrap) }
    ()
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
            gmp::mpz_mul_ui(q_mut, q_const, n2);
            gmp::mpz_mul_ui(q_mut, q_const, n2);
            gmp::mpz_mul_ui(q_mut, q_const, n2);
            gmp::mpz_add_ui(&mut pqt.t as *mut mpz_t, &pqt.t as *const mpz_t, A);
            let mut b_mpz = allocate_mpz(B);
            gmp::mpz_mul_ui(&mut b_mpz as *mut mpz_t, &b_mpz as *const mpz_t, n2);
            mpz_mul(pqt.t, pqt.t, pqt.p);
            if (n2 & 1) == 1 {
                gmp::mpz_neg(&mut pqt.t as *mut mpz_t, &pqt.t as *const mpz_t);
            }
        } else {
            let m = (n1 + n2) / 2;
            // single thread
            let mut res1 = i_compute_pqt(n1, m); // res1 is used as a temp buffer to reduce mem
            let mut res2 = i_compute_pqt(m, n2);
            mpz_mul(pqt.p, res1.p, res2.p);
            mpz_mul(pqt.q, res1.q, res2.q);
            let t_1 = allocate_mpz(0);
            let t_2 = allocate_mpz(0);
            mpz_mul(t_1, res1.t, res2.q);
            mpz_mul(t_2, res1.p, res2.t);
            mpz_add(pqt.t, t_1, t_2);

            gmp::mpz_clear(&mut res1.p);
            gmp::mpz_clear(&mut res1.q);
            gmp::mpz_clear(&mut res1.t);
            gmp::mpz_clear(&mut res2.p);
            gmp::mpz_clear(&mut res2.q);
            gmp::mpz_clear(&mut res2.t);
        }
    }
    pqt
}

#[async_recursion::async_recursion]
async fn compute_pqt(n1: u64, n2: u64) -> PQT {
    // async unsafe fn compute_pqt(a: mpz_t, b: mpz_t, c3_24: mpz_t, n1: u64, n2: u64) -> PQT {
    // println!("compute_pqt called {} {}", n1, n2);
    let mut pqt: PQT = PQT {
        p: allocate_mpz(0),
        q: allocate_mpz(0),
        t: allocate_mpz(0),
    };
    unsafe {
        if n1 + 1 == n2 {
            gmp::mpz_set_ui(&mut pqt.p as *mut mpz_t, 2 * n2 - 1);
            gmp::mpz_mul_ui(&mut pqt.p as *mut mpz_t, &pqt.p as *const mpz_t, 6 * n2 - 1);
            gmp::mpz_mul_ui(&mut pqt.p as *mut mpz_t, &pqt.p as *const mpz_t, 6 * n2 - 5);
            gmp::mpz_set_ui(&mut pqt.q as *mut mpz_t, C3_24);
            gmp::mpz_mul_ui(&mut pqt.q as *mut mpz_t, &pqt.q as *const mpz_t, n2);
            gmp::mpz_mul_ui(&mut pqt.q as *mut mpz_t, &pqt.q as *const mpz_t, n2);
            gmp::mpz_mul_ui(&mut pqt.q as *mut mpz_t, &pqt.q as *const mpz_t, n2);
            gmp::mpz_add_ui(&mut pqt.t as *mut mpz_t, &pqt.t as *const mpz_t, A);
            let mut b_mpz = allocate_mpz(B);
            gmp::mpz_mul_ui(&mut b_mpz as *mut mpz_t, &b_mpz as *const mpz_t, n2);
            mpz_mul(pqt.t, pqt.t, pqt.p);
            if (n2 & 1) == 1 {
                gmp::mpz_neg(&mut pqt.t as *mut mpz_t, &pqt.t as *const mpz_t);
            }
        } else {
            let m = (n1 + n2) / 2;
            let mut res1: PQT;
            let mut res2: PQT;
            if n2 - n1 < 10u64.pow(5) {
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
            if n2 - n1 > 1000000 {
                println!("{}", n2);
            }
            // p = res1 p * res2 p
            let wrap_p = WrappedMpzTri {
                a: pqt.p,
                b: res1.p,
                c: res2.p,
            };
            let p_thread = tokio::task::spawn_blocking(move || wrap_mul(wrap_p));
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
            let q_thread = tokio::task::spawn_blocking(move || wrap_mul(wrap_q));
            // let q_thread = tokio::spawn(move || wrap_mul(wrap_q));
            //let q_thread = tokio::spawn(move || {
            //    wrap_mul(wrap_q);
            //});
            let (t_1, t_2) = {
                let t_1_val = allocate_mpz(0);
                let t_2_val = allocate_mpz(0);
                let t_1 = WrappedMpz { a: t_1_val };
                let t_2 = WrappedMpz { a: t_2_val };
                (t_1, t_2)
            };

            //let t_1 = allocate_mpz(0);
            let t_1_wrap = WrappedMpzTri {
                a: t_1.a,
                b: res1.t,
                c: res2.q,
            };
            let t_1_handle = tokio::task::spawn_blocking(move || wrap_mul(t_1_wrap));
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
            let t_2_handle = tokio::task::spawn_blocking(move || wrap_mul(t_2_wrap));
            // let t_2_thread = tokio::spawn(move || wrap_mul(wrap_t_2));
            //let t_2_thread = tokio::spawn(move || {
            //    wrap_mul(t_2_wrap);
            //});
            let _ = t_1_handle.await.unwrap();
            let _ = t_2_handle.await.unwrap();
            mpz_add(pqt.t, t_1.a, t_2.a);
            let _ = p_thread.await.unwrap();
            let _ = q_thread.await.unwrap();
            gmp::mpz_clear(&mut res1.p);
            gmp::mpz_clear(&mut res1.q);
            gmp::mpz_clear(&mut res1.t);
            gmp::mpz_clear(&mut res2.p);
            gmp::mpz_clear(&mut res2.q);
            gmp::mpz_clear(&mut res2.t);
        }
    }
    pqt
}

#[tokio::main(flavor = "multi_thread", worker_threads = 30)]
async fn main() {
    let digits = env::args().nth(1).unwrap().parse::<u32>().unwrap();
    println!("Computing {} digits", digits);
    let prec = (digits * 10u32.ilog2()) as u64;
    // let comp: Chudnovsky = Chudnovsky::default();
    let digits_per_term = (53360f64.powf(3f64).ln()) / 10f64.ln();
    let n = digits as f64 / digits_per_term;
    println!("{} {} {}", digits_per_term, n, prec);
    unsafe {
        let mut c3_24 = allocate_mpz(C);
        gmp::mpz_pow_ui(&mut c3_24 as *mut mpz_t, &c3_24 as *const mpz_t, 3);
        gmp::mpz_divexact_ui(&mut c3_24 as *mut mpz_t, &c3_24 as *const mpz_t, 24);
        let pqt: PQT = compute_pqt(0u64, n as u64).await;
        println!("pqt done");
        let mut pi = allocate_mpf(0, prec);
        let mut e = allocate_mpf(E, 0u64); // e
        let mut q = allocate_mpf(0, 0u64);
        let mut t = allocate_mpf(0, 0u64);
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
            &allocate_mpf(D, 0u64) as *const mpf_t, // d
        );
        println!("mul2 done");
        // e is not representative here. just saving mem
        gmp::mpf_add(&mut e as *mut mpf_t, &q as *const mpf_t, &t as *const mpf_t);
        gmp::mpf_div(
            &mut pi as *mut mpf_t,
            &pi as *const mpf_t,
            &e as *const mpf_t,
        );
        gmp::mpf_div(
            &mut pi as *mut mpf_t,
            &pi as *const mpf_t,
            &allocate_mpf(A, 0u64) as *const mpf_t,
        );
        let mut expptr: i64 = 0;
        println!("computed, making string");
        //let printout = CStr::from_ptr(gmp::mpf_get_str(
        //    std::ptr::null_mut(),
        //    &mut expptr,
        //    10i32,
        //    digits as usize,
        //    &pi as *const mpf_t,
        //));
        //println!("{}", printout.to_str().unwrap());
    }
}
