use std::time;
use rand::Rng;

fn main(){
    let mut rng = rand::thread_rng();
    let mut tempa: Vec<u8> = Vec::new();
    let mut tempb: Vec<u8> = Vec::new();
    for _ in 0..512{
        tempa.push(rng.gen());
        tempb.push(rng.gen());
    }
    
    let mut start = time::Instant::now();
    array(&tempa[..], &tempb[..]);
    println!("time 512, 512 array: {:?}ns", start.elapsed().as_nanos());
    start = time::Instant::now();
    array(&tempa[..], &tempb[..128]);
    println!("time 512, 128 array: {:?}ns", start.elapsed().as_nanos());
    start = time::Instant::now();
    array(&tempa[..128], &tempb[..]);
    println!("time 128, 512 array: {:?}ns", start.elapsed().as_nanos());
    start = time::Instant::now();
    array(&tempa[..128], &tempb[..128]);
    println!("time 128, 128 array: {:?}ns", start.elapsed().as_nanos());
    start = time::Instant::now();
    vector(&tempa[..], &tempb[..]);
    println!("time 512, 512 vector: {:?}ns", start.elapsed().as_nanos());
    start = time::Instant::now();
    vector(&tempa[..], &tempb[..128]);
    println!("time 512, 128 vector: {:?}ns", start.elapsed().as_nanos());
    start = time::Instant::now();
    vector(&tempa[..128], &tempb[..]);
    println!("time 128, 512 vector: {:?}ns", start.elapsed().as_nanos());
    start = time::Instant::now();
    vector(&tempa[..128], &tempb[..128]);
    println!("time 128, 128 vector: {:?}ns", start.elapsed().as_nanos());
    start = time::Instant::now();

}


fn array<'a>(a: &'a[u8], b: &'a[u8]){
    let mut c: [u8; 1024] = [0; 1024];
    let mut i: usize = 0;
    for j in a{
        c[i] = *j;
        i += 1;
    }
    for j in b{
        c[i] = *j;
        i += 1
    }
    let res = &c[0..i];
}

fn vector<'a>(a: &'a[u8], b: &'a[u8]){
    let c = [a, b].join(&0);
}
