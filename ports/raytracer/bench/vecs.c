#include <stdio.h>
typedef struct { double x, y, z; } v3;
static v3 add(v3 a, v3 b) { v3 r = { a.x + b.x, a.y + b.y, a.z + b.z }; return r; }
int main(void) {
  volatile double k = 0.5;
  v3 v = {0, 0, 0};
  for (long i = 0; i < 10000000; i++) { v3 d = { k, k / 2, k / 4 }; v = add(v, d); }
  printf("%.1f\n", v.x + v.y + v.z);
  return 0;
}
