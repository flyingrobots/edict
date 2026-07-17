(module
  (memory (export "memory") 1)

  (func (export "cabi_realloc")
    (param i32 i32 i32 i32)
    (result i32)
    i32.const 0
  )

  (func (export "lower") (param i32) (result i32)
    i32.const 0
  )

  (func (export "cabi_post_lower") (param i32))

  (func $start
    (loop $forever
      br $forever
    )
  )
  (start $start)
)
