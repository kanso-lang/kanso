#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <math.h>
#include <stdint.h>

/* ABI shared with emitted LLVM IR: %KValue = type { i64, i64 } */
typedef struct { long long tag; long long payload; } KValue;

enum { K_INT, K_FLOAT, K_TRUE, K_FALSE, K_NONE, K_ERR, K_STR, K_REC, K_DESC };

typedef struct { long len; char* data; } KStr;
typedef struct { long long type_id; long long nfields; KValue* fields; } KRec;
typedef struct KDesc KDesc;
struct KDesc { int dtag; KStr* text; KDesc* a; KDesc* b; };

static void* k_alloc(size_t n) {
    void* p = malloc(n);
    if (!p) { fputs("out of memory\n", stderr); exit(1); }
    return p;
}

void k_die(const char* msg) {
    fprintf(stderr, "error[runtime]: %s\n", msg);
    exit(1);
}

static long long k_ptr(void* p) { return (long long)(intptr_t)p; }
static KStr* k_as_str(KValue v) { return (KStr*)(intptr_t)v.payload; }
static KRec* k_as_rec(KValue v) { return (KRec*)(intptr_t)v.payload; }
static KDesc* k_as_desc(KValue v) { return (KDesc*)(intptr_t)v.payload; }
static KValue* k_as_boxed(KValue v) { return (KValue*)(intptr_t)v.payload; }

static double k_as_f(KValue v) { double d; memcpy(&d, &v.payload, 8); return d; }

KValue k_float(double d) {
    KValue v; v.tag = K_FLOAT; memcpy(&v.payload, &d, 8); return v;
}

KValue k_int(long long i) { KValue v; v.tag = K_INT; v.payload = i; return v; }
KValue k_bool(long long b) { KValue v; v.tag = b ? K_TRUE : K_FALSE; v.payload = 0; return v; }
KValue k_none(void) { KValue v; v.tag = K_NONE; v.payload = 0; return v; }

KValue k_str_n(const char* data, long long len) {
    KStr* s = k_alloc(sizeof(KStr));
    s->len = (long)len;
    s->data = k_alloc(len + 1);
    memcpy(s->data, data, len);
    s->data[len] = 0;
    KValue v; v.tag = K_STR; v.payload = k_ptr(s); return v;
}

static KValue k_str(const char* data) { return k_str_n(data, (long long)strlen(data)); }

long long k_not_failure(KValue v) { return v.tag != K_ERR && v.tag != K_NONE; }

KValue k_err(KValue reason) {
    if (!k_not_failure(reason)) return reason;
    KValue* boxed = k_alloc(sizeof(KValue));
    *boxed = reason;
    KValue v; v.tag = K_ERR; v.payload = k_ptr(boxed); return v;
}

KValue k_rec(long long type_id, long long n, KValue* args) {
    for (long long i = 0; i < n; i++) if (!k_not_failure(args[i])) return args[i];
    KRec* r = k_alloc(sizeof(KRec));
    r->type_id = type_id;
    r->nfields = n;
    r->fields = k_alloc(sizeof(KValue) * n);
    memcpy(r->fields, args, sizeof(KValue) * n);
    KValue v; v.tag = K_REC; v.payload = k_ptr(r); return v;
}

KValue k_field(KValue v, long long i) { return k_as_rec(v)->fields[i]; }
KValue k_err_inner(KValue v) { return *k_as_boxed(v); }

/* pattern checks: nonzero on match */
long long k_check_tag(KValue v, long long tag) { return v.tag == tag; }
long long k_check_int(KValue v, long long n) { return v.tag == K_INT && v.payload == n; }
long long k_check_rec(KValue v, long long type_id, long long nfields) {
    return v.tag == K_REC && k_as_rec(v)->type_id == type_id
        && k_as_rec(v)->nfields == nfields;
}
long long k_check_bool(KValue v) { return v.tag == K_TRUE || v.tag == K_FALSE; }

KValue k_concat(KValue a, KValue b) {
    if (!k_not_failure(a)) return a;
    if (!k_not_failure(b)) return b;
    KStr* sa = k_as_str(a);
    KStr* sb = k_as_str(b);
    KStr* s = k_alloc(sizeof(KStr));
    s->len = sa->len + sb->len;
    s->data = k_alloc(s->len + 1);
    memcpy(s->data, sa->data, sa->len);
    memcpy(s->data + sa->len, sb->data, sb->len);
    s->data[s->len] = 0;
    KValue v; v.tag = K_STR; v.payload = k_ptr(s); return v;
}

extern const char* k_type_name(long long type_id);

KValue k_render(KValue v, long long quote) {
    char buf[64];
    switch (v.tag) {
        case K_INT:
            snprintf(buf, sizeof buf, "%lld", v.payload);
            return k_str(buf);
        case K_FLOAT: {
            double d = k_as_f(v);
            if (d == floor(d) && fabs(d) < 1e15 && isfinite(d)) {
                snprintf(buf, sizeof buf, "%.1f", d);
                return k_str(buf);
            }
            for (int prec = 1; prec <= 17; prec++) {
                snprintf(buf, sizeof buf, "%.*g", prec, d);
                if (strtod(buf, NULL) == d) break;
            }
            return k_str(buf);
        }
        case K_TRUE: return k_str("true");
        case K_FALSE: return k_str("false");
        case K_NONE: return k_str("none");
        case K_ERR: return k_concat(k_str("err "), k_render(k_err_inner(v), 1));
        case K_STR:
            if (!quote) return v;
            return k_concat(k_concat(k_str("\""), v), k_str("\""));
        case K_REC: {
            KRec* r = k_as_rec(v);
            KValue out = k_str(k_type_name(r->type_id));
            for (long long i = 0; i < r->nfields; i++) {
                out = k_concat(out, k_str(" "));
                out = k_concat(out, k_render(r->fields[i], 1));
            }
            return out;
        }
        case K_DESC: return k_str("<description>");
    }
    return k_str("<value>");
}

static long long k_eq(KValue a, KValue b) {
    if (a.tag != b.tag) return 0;
    switch (a.tag) {
        case K_INT: return a.payload == b.payload;
        case K_FLOAT: return k_as_f(a) == k_as_f(b);
        case K_TRUE: case K_FALSE: case K_NONE: return 1;
        case K_STR: {
            KStr* sa = k_as_str(a);
            KStr* sb = k_as_str(b);
            return sa->len == sb->len && memcmp(sa->data, sb->data, sa->len) == 0;
        }
        case K_REC: {
            KRec* ra = k_as_rec(a);
            KRec* rb = k_as_rec(b);
            if (ra->type_id != rb->type_id) return 0;
            for (long long i = 0; i < ra->nfields; i++) {
                if (!k_eq(ra->fields[i], rb->fields[i])) return 0;
            }
            return 1;
        }
        default: return 0;
    }
}

long long k_check_str(KValue v, const char* data, long long len) {
    if (v.tag != K_STR) return 0;
    KStr* s = k_as_str(v);
    return s->len == len && memcmp(s->data, data, len) == 0;
}

KValue k_add(KValue a, KValue b) {
    if (!k_not_failure(a)) return a;
    if (!k_not_failure(b)) return b;
    if (a.tag == K_INT && b.tag == K_INT) {
        long long r;
        if (__builtin_add_overflow(a.payload, b.payload, &r)) return k_err(k_str("integer overflow"));
        return k_int(r);
    }
    if (a.tag == K_FLOAT && b.tag == K_FLOAT) return k_float(k_as_f(a) + k_as_f(b));
    k_die("`+` is not defined for these values");
    return k_none();
}

KValue k_sub(KValue a, KValue b) {
    if (!k_not_failure(a)) return a;
    if (!k_not_failure(b)) return b;
    if (a.tag == K_INT && b.tag == K_INT) {
        long long r;
        if (__builtin_sub_overflow(a.payload, b.payload, &r)) return k_err(k_str("integer overflow"));
        return k_int(r);
    }
    if (a.tag == K_FLOAT && b.tag == K_FLOAT) return k_float(k_as_f(a) - k_as_f(b));
    k_die("`-` is not defined for these values");
    return k_none();
}

KValue k_mul(KValue a, KValue b) {
    if (!k_not_failure(a)) return a;
    if (!k_not_failure(b)) return b;
    if (a.tag == K_INT && b.tag == K_INT) {
        long long r;
        if (__builtin_mul_overflow(a.payload, b.payload, &r)) return k_err(k_str("integer overflow"));
        return k_int(r);
    }
    if (a.tag == K_FLOAT && b.tag == K_FLOAT) return k_float(k_as_f(a) * k_as_f(b));
    k_die("`*` is not defined for these values");
    return k_none();
}

KValue k_div(KValue a, KValue b) {
    if (!k_not_failure(a)) return a;
    if (!k_not_failure(b)) return b;
    if (a.tag == K_INT && b.tag == K_INT) {
        if (b.payload == 0) return k_err(k_str("division by zero"));
        return k_int(a.payload / b.payload);
    }
    if (a.tag == K_FLOAT && b.tag == K_FLOAT) {
        if (k_as_f(b) == 0.0) return k_err(k_str("division by zero"));
        return k_float(k_as_f(a) / k_as_f(b));
    }
    k_die("`/` is not defined for these values");
    return k_none();
}

static int k_order(KValue a, KValue b) {
    if (a.tag == K_INT && b.tag == K_INT) return (a.payload > b.payload) - (a.payload < b.payload);
    if (a.tag == K_FLOAT && b.tag == K_FLOAT) {
        double x = k_as_f(a);
        double y = k_as_f(b);
        return (x > y) - (x < y);
    }
    if (a.tag == K_STR && b.tag == K_STR) {
        KStr* sa = k_as_str(a);
        KStr* sb = k_as_str(b);
        long n = sa->len < sb->len ? sa->len : sb->len;
        int c = memcmp(sa->data, sb->data, n);
        if (c) return c > 0 ? 1 : -1;
        return (sa->len > sb->len) - (sa->len < sb->len);
    }
    k_die("comparison requires two values of one comparable type");
    return 0;
}

KValue k_cmp(KValue a, KValue b, long long op) {
    if (!k_not_failure(a)) return a;
    if (!k_not_failure(b)) return b;
    if (op == 0) return k_bool(k_eq(a, b));
    if (op == 1) return k_bool(!k_eq(a, b));
    int c = k_order(a, b);
    switch (op) {
        case 2: return k_bool(c < 0);
        case 3: return k_bool(c <= 0);
        case 4: return k_bool(c > 0);
        default: return k_bool(c >= 0);
    }
}

KValue k_desc_print(KValue text) {
    if (!k_not_failure(text)) return text;
    if (text.tag != K_STR) k_die("print takes a string; interpolate instead");
    KDesc* d = k_alloc(sizeof(KDesc));
    d->dtag = 0; d->text = k_as_str(text); d->a = d->b = NULL;
    KValue v; v.tag = K_DESC; v.payload = k_ptr(d); return v;
}

KValue k_seq(KValue a, KValue b) {
    if (!k_not_failure(a)) return a;
    if (!k_not_failure(b)) return b;
    if (a.tag != K_DESC || b.tag != K_DESC) k_die("`>>` sequences two effect descriptions");
    KDesc* d = k_alloc(sizeof(KDesc));
    d->dtag = 1; d->text = NULL; d->a = k_as_desc(a); d->b = k_as_desc(b);
    KValue v; v.tag = K_DESC; v.payload = k_ptr(d); return v;
}

static void k_exec(KDesc* d) {
    if (d->dtag == 0) {
        fwrite(d->text->data, 1, d->text->len, stdout);
        fputc('\n', stdout);
    } else {
        k_exec(d->a);
        k_exec(d->b);
    }
}

long long k_truthy(KValue v) {
    if (v.tag == K_TRUE) return 1;
    if (v.tag == K_FALSE) return 0;
    k_die("an if condition is true or false");
    return 0;
}

extern KValue d_main_0(void);

int main(void) {
    KValue v = d_main_0();
    if (v.tag == K_DESC) { k_exec(k_as_desc(v)); return 0; }
    if (v.tag == K_ERR) {
        KValue r = k_render(k_err_inner(v), 1);
        fprintf(stderr, "error[endpoint]: unhandled err reached main: %s\n", k_as_str(r)->data);
        return 1;
    }
    if (v.tag == K_NONE) {
        fputs("error[endpoint]: unhandled none reached main\n", stderr);
        return 1;
    }
    return 0;
}
