# pi-thing (wip)
Pi solver in rust, async unsafe and fast

todo
- [x] make work
- [x] multi thread
- [x] add pell series
- [ ] move to rayon
- [ ] move to flint
- [ ] implement fft myself?
- [ ] use newton raphson instead of pell
- [ ] optimize memory
- [ ] clean up main code


```sh
git clone https://github.com/Ednaordinary/pi-thing
cd pi-thing
git clone https://github.com/alex-ozdemir/flint-rs
export CC="gcc -Wno-error=implicit-function-declaration -Wno-error=incompatible-pointer-types -Wno-error=int-conversion -Wno-error=return-mismatch -Dnoreturn="
cargo build -r
```
