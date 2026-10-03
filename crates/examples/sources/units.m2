import c_std::io;

unit Point {
    pub x:i32;
    pub y:i32;
};

func main() -> i32 {
    var point = Point { 10, 20 };
    point.x = 30;
    c_std::io::printf(const_c_str("point=(%d,%d)\n"), point.x, point.y);
    return 0;
}
