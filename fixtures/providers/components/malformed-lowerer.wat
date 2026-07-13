(module
  (memory (export "memory") 2)
  (global $next (mut i32) (i32.const 4096))

  (func $realloc
    (export "cabi_realloc")
    (param $old i32)
    (param $old-size i32)
    (param $align i32)
    (param $new-size i32)
    (result i32)
    (local $pointer i32)
    global.get $next
    local.get $align
    i32.const 1
    i32.sub
    i32.add
    local.get $align
    i32.const 1
    i32.sub
    i32.const -1
    i32.xor
    i32.and
    local.tee $pointer
    local.get $new-size
    i32.add
    global.set $next
    local.get $pointer
  )

  (func (export "lower") (param i32) (result i32)
    i32.const 1024
  )

  (func (export "cabi_post_lower") (param i32))

  ;; A lowering result has only ok (0) and err (1) arms.
  (data (i32.const 1024) "\02")
)
