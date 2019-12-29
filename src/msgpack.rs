use emacs::{defun, Env, IntoLisp, Result, Value, Vector};
use rmpv::Value as RValue;

struct MPValue(RValue);

impl IntoLisp<'_> for MPValue {
    fn into_lisp(self, env: &Env) -> Result<Value> {
        match self.0 {
            RValue::Nil => env.intern("nil"),
            RValue::Boolean(b) => env.intern(if b { "t" } else { "nil" }),
            RValue::Integer(i) => i.as_i64().into_lisp(env),
            RValue::F32(f) => (f as f64).into_lisp(env),
            RValue::F64(f) => f.into_lisp(env),
            RValue::String(s) => s.as_str().into_lisp(env),
            RValue::Binary(v) => env.call(
                "vector",
                &v.into_iter()
                    .map(|b| b.into_lisp(env).unwrap())
                    .collect::<Vec<_>>(),
            ),
            RValue::Array(arr) => env.call(
                "vector",
                &arr.into_iter()
                    .map(|elt| MPValue(elt).into_lisp(env).unwrap())
                    .collect::<Vec<_>>(),
            ),
            RValue::Map(m) => env.list(
                &m.into_iter()
                    .map(|(k, v)| {
                        env.list((
                            MPValue(k).into_lisp(env).unwrap(),
                            MPValue(v).into_lisp(env).unwrap(),
                        ))
                        .unwrap()
                    })
                    .collect::<Vec<_>>(),
            ),
            RValue::Ext(typ, arr) => {
                let ext = env.intern("ext")?;
                let typ = typ.into_lisp(env)?;
                let arr = env.list(
                    &arr.into_iter()
                        .map(|b| b.into_lisp(env).unwrap())
                        .collect::<Vec<_>>(),
                )?;

                env.list((ext, typ, arr))
            }
        }
    }
}

fn alistp(env: &Env, val: Value) -> Result<bool> {
    if val.is_not_nil() {
        let car = env.call("car", [val])?;

        if env.call("consp", [car])?.is_not_nil() {
            let cdr = env.call("cdr", [val])?;
            alistp(env, cdr)
        } else {
            Ok(false)
        }
    } else {
        Ok(true)
    }
}

fn encode_value(env: &Env, val: Value) -> Result<RValue> {
    let t = env.intern("t")?;

    if val.eq(t) {
        Ok(RValue::from(true))
    } else if !val.is_not_nil() {
        Ok(RValue::Nil)
    } else if env.call("integerp", [val])?.is_not_nil() {
        Ok(RValue::from(val.into_rust::<i64>()?))
    } else if env.call("floatp", [val])?.is_not_nil() {
        Ok(RValue::from(val.into_rust::<f64>()?))
    } else if env.call("stringp", [val])?.is_not_nil() {
        Ok(RValue::from(val.into_rust::<String>()?))
    } else if env.call("vectorp", [val])?.is_not_nil() {
        let lisp_vector = Vector(val);
        let mut vector = Vec::new();

        for i in 0..lisp_vector.size()? {
            vector.push(encode_value(env, lisp_vector.get(i)?)?)
        }

        Ok(RValue::from(vector))
    } else if env.call("listp", [val])?.is_not_nil() {
        let mut list = val;

        if alistp(env, val.clone())? {
            let mut vector = Vec::new();

            while list.is_not_nil() {
                let caar = env.call("caar", [list])?;
                let cdar = env.call("cdar", [list])?;
                list = env.call("cdr", [list])?;

                vector.push((encode_value(env, caar)?, encode_value(env, cdar)?));
            }

            Ok(RValue::from(vector))
        } else {
            let mut vector = Vec::new();

            while list.is_not_nil() {
                let car = env.call("car", [list])?;
                list = env.call("cdr", [list])?;

                vector.push(encode_value(env, car)?);
            }
            Ok(RValue::from(vector))
        }
    } else {
        Ok(RValue::Nil)
    }
}

#[defun]
fn read<'a>(env: &'a Env, val: Value) -> Result<Value<'a>> {
    let mut list = val;
    let mut buf = Vec::new();

    while list.is_not_nil() {
        let car = env.call("car", [list])?;
        list = env.call("cdr", [list])?;
        buf.push(car.into_rust::<u8>()?);
    }
    let val = rmpv::decode::read_value(&mut &buf[..])?;

    MPValue(val).into_lisp(env)
}

#[defun]
fn read_string(env: &Env, string: String) -> Result<Value> {
    env.message(format!("string: {:?}", string.as_bytes()))?;
    let val = rmpv::decode::read_value(&mut string.as_bytes())?;

    MPValue(val).into_lisp(env)
}

#[defun]
fn encode<'a>(env: &'a Env, val: Value) -> Result<Value<'a>> {
    let val = encode_value(env, val)?;
    let mut buf = Vec::new();
    rmpv::encode::write_value(&mut buf, &val)?;
    env.list(
        &buf.into_iter()
            .map(|b| b.into_lisp(env).unwrap())
            .collect::<Vec<_>>(),
    )
}
