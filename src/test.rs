#[cfg(test)]
mod tests {
  use crate::swc::SWC;

  #[test]
  fn parse_cjs_exports_case_1() {
    let source = r#"
      const c = 'c'
      Object.defineProperty(exports, 'a', { value: true })
      Object.defineProperty(exports, 'b', { get: () => true })
      Object.defineProperty(exports, c, { get() { return true } })
      Object.defineProperty(exports, 'd', { "value": true })
      Object.defineProperty(exports, 'e', { "get": () => true })
      Object.defineProperty(exports, 'f', {})
      Object.defineProperty(module.exports, '__esModule', { value: true })
    "#;
    let swc = SWC::parse("index.cjs", source).expect("could not parse module");
    let (exports, _) = swc
      .parse_cjs_exports("development", false)
      .expect("could not parse exports");
    assert_eq!(exports.join(","), "a,b,c,d,e,__esModule")
  }

  #[test]
  fn parse_cjs_exports_case_2() {
    let source = r#"
      const alas = true
      const obj = { bar: 123 }
      Object.defineProperty(exports, 'nope', { value: true })
      Object.defineProperty(module, 'exports', { value: { alas, foo: 'bar', ...obj, ...require('a'), ...require('b') } })
    "#;
    let swc = SWC::parse("index.cjs", source).expect("could not parse module");
    let (exports, reexports) = swc
      .parse_cjs_exports("development", false)
      .expect("could not parse exports");
    assert_eq!(exports.join(","), "alas,foo,bar");
    assert_eq!(reexports.join(","), "a,b");
  }

  #[test]
  fn parse_cjs_exports_case_3() {
    let source = r#"
      const alas = true
      const obj = { bar: 1 }
      obj.meta = 1
      Object.assign(module.exports, { alas, foo: 'bar', ...obj }, { ...require('a') }, require('b'))
    "#;
    let swc = SWC::parse("index.cjs", source).expect("could not parse module");
    let (exports, reexports) = swc
      .parse_cjs_exports("development", false)
      .expect("could not parse exports");
    assert_eq!(exports.join(","), "alas,foo,bar,meta");
    assert_eq!(reexports.join(","), "a,b");
  }

  #[test]
  fn parse_cjs_exports_case_4() {
    let source = r#"
      Object.assign(module.exports, { foo: 'bar', ...require('lib') })
      Object.assign(module, { exports: { nope: true } })
    "#;
    let swc = SWC::parse("index.cjs", source).expect("could not parse module");
    let (exports, reexports) = swc
      .parse_cjs_exports("development", false)
      .expect("could not parse exports");
    assert_eq!(exports.join(","), "nope");
    assert_eq!(reexports.join(","), "");
  }

  #[test]
  fn parse_cjs_exports_case_5() {
    let source = r#"
      exports.foo = 'bar'
      module.exports.bar = 123
    "#;
    let swc = SWC::parse("index.cjs", source).expect("could not parse module");
    let (exports, _) = swc
      .parse_cjs_exports("development", false)
      .expect("could not parse exports");
    assert_eq!(exports.join(","), "foo,bar");
  }

  #[test]
  fn parse_cjs_exports_case_6() {
    let source = r#"
      const alas = true
      const obj = { boom: 1 }
      obj.coco = 1
      exports.foo = 'bar'
      module.exports.bar = 123
      module.exports = { alas,  ...obj, ...require('a'), ...require('b') }
    "#;
    let swc = SWC::parse("index.cjs", source).expect("could not parse module");
    let (exports, reexports) = swc
      .parse_cjs_exports("development", false)
      .expect("could not parse exports");
    assert_eq!(exports.join(","), "alas,boom,coco");
    assert_eq!(reexports.join(","), "a,b");
  }

  #[test]
  fn parse_cjs_exports_case_7() {
    let source = r#"
      exports['foo'] = 'bar'
      module['exports']['bar'] = 123
    "#;
    let swc = SWC::parse("index.cjs", source).expect("could not parse module");
    let (exports, _) = swc
      .parse_cjs_exports("development", false)
      .expect("could not parse exports");
    assert_eq!(exports.join(","), "foo,bar");
  }

  #[test]
  fn parse_cjs_exports_case_8() {
    let source = r#"
      module.exports = function() {}
      module.exports.foo = 'bar';
    "#;
    let swc = SWC::parse("index.cjs", source).expect("could not parse module");
    let (exports, _) = swc
      .parse_cjs_exports("development", false)
      .expect("could not parse exports");
    assert_eq!(exports.join(","), "foo");
  }

  #[test]
  fn parse_cjs_exports_case_9() {
    let source = r#"
      module.exports = require("lib")
    "#;
    let swc = SWC::parse("index.cjs", source).expect("could not parse module");
    let (_, reexports) = swc
      .parse_cjs_exports("development", false)
      .expect("could not parse exports");
    assert_eq!(reexports.join(","), "lib");
  }

  #[test]
  fn parse_cjs_exports_case_9_1() {
    let source = r#"
      var lib = require("lib")
      module.exports = lib
    "#;
    let swc = SWC::parse("index.cjs", source).expect("could not parse module");
    let (_, reexports) = swc
      .parse_cjs_exports("development", false)
      .expect("could not parse exports");
    assert_eq!(reexports.join(","), "lib");
  }

  #[test]
  fn parse_cjs_exports_case_10() {
    let source = r#"
      function Module() {}
      Module.foo = 'bar'
      module.exports = Module
    "#;
    let swc = SWC::parse("index.cjs", source).expect("could not parse module");
    let (exports, _) = swc
      .parse_cjs_exports("development", false)
      .expect("could not parse exports");
    assert_eq!(exports.join(","), "foo");
  }

  #[test]
  fn parse_cjs_exports_case_10_1() {
    let source = r#"
      let Module = function () {}
      Module.foo = 'bar'
      module.exports = Module
    "#;
    let swc = SWC::parse("index.cjs", source).expect("could not parse module");
    let (exports, _) = swc
      .parse_cjs_exports("development", false)
      .expect("could not parse exports");
    assert_eq!(exports.join(","), "foo");
  }

  #[test]
  fn parse_cjs_exports_case_10_2() {
    let source = r#"
      let Module = () => {}
      Module.foo = 'bar'
      module.exports = Module
    "#;
    let swc = SWC::parse("index.cjs", source).expect("could not parse module");
    let (exports, _) = swc
      .parse_cjs_exports("development", false)
      .expect("could not parse exports");
    assert_eq!(exports.join(","), "foo");
  }

  #[test]
  fn parse_cjs_exports_case_11() {
    let source = r#"
      class Module {
        static foo = 'bar'
        static greet() {}
        alas = true
        boom() {}
      }
      module.exports = Module
    "#;
    let swc = SWC::parse("index.cjs", source).expect("could not parse module");
    let (exports, _) = swc
      .parse_cjs_exports("development", false)
      .expect("could not parse exports");
    assert_eq!(exports.join(","), "foo,greet");
  }

  #[test]
  fn parse_cjs_exports_case_12() {
    let source = r#"
      (function() {
        module.exports = { foo: 'bar' }
      })()
    "#;
    let swc = SWC::parse("index.cjs", source).expect("could not parse module");
    let (exports, _) = swc
      .parse_cjs_exports("development", false)
      .expect("could not parse exports");
    assert_eq!(exports.join(","), "foo");
  }

  #[test]
  fn parse_cjs_exports_case_12_1() {
    let source = r#"
      (() => {
        module.exports = { foo: 'bar' }
      })()
    "#;
    let swc = SWC::parse("index.cjs", source).expect("could not parse module");
    let (exports, _) = swc
      .parse_cjs_exports("development", false)
      .expect("could not parse exports");
    assert_eq!(exports.join(","), "foo");
  }

  #[test]
  fn parse_cjs_exports_case_12_2() {
    let source = r#"
      (function() {
        module.exports = { foo: 'bar' }
      }())
    "#;
    let swc = SWC::parse("index.cjs", source).expect("could not parse module");
    let (exports, _) = swc
      .parse_cjs_exports("development", false)
      .expect("could not parse exports");
    assert_eq!(exports.join(","), "foo");
  }

  #[test]
  fn parse_cjs_exports_case_12_3() {
    let source = r#"
      ~function() {
        module.exports = { foo: 'bar' }
      }()
    "#;
    let swc = SWC::parse("index.cjs", source).expect("could not parse module");
    let (exports, _) = swc
      .parse_cjs_exports("development", false)
      .expect("could not parse exports");
    assert_eq!(exports.join(","), "foo");
  }

  #[test]
  fn parse_cjs_exports_case_12_4() {
    let source = r#"
      let es = { foo: 'bar' };
      (function() {
        module.exports = es
      })()
    "#;
    let swc = SWC::parse("index.cjs", source).expect("could not parse module");
    let (exports, _) = swc
      .parse_cjs_exports("development", false)
      .expect("could not parse exports");
    assert_eq!(exports.join(","), "foo");
  }

  #[test]
  fn parse_cjs_exports_case_13() {
    let source = r#"
      {
        module.exports = { foo: 'bar' }
      }
    "#;
    let swc = SWC::parse("index.cjs", source).expect("could not parse module");
    let (exports, _) = swc
      .parse_cjs_exports("development", false)
      .expect("could not parse exports");
    assert_eq!(exports.join(","), "foo");
  }

  #[test]
  fn parse_cjs_exports_case_13_1() {
    let source = r#"
      const obj1 = { foo: 'bar' }
      {
        const obj2 = { bar: 123 }
        module.exports = { ...obj1, ...obj2 }
      }
    "#;
    let swc = SWC::parse("index.cjs", source).expect("could not parse module");
    let (exports, _) = swc
      .parse_cjs_exports("development", false)
      .expect("could not parse exports");
    assert_eq!(exports.join(","), "foo,bar");
  }

  #[test]
  fn parse_cjs_exports_case_14() {
    let source = r#"
      if (process.env.NODE_ENV === 'development') {
        module.exports = { foo: 'bar' }
      }
    "#;
    let swc = SWC::parse("index.cjs", source).expect("could not parse module");
    let (exports, _) = swc
      .parse_cjs_exports("development", false)
      .expect("could not parse exports");
    assert_eq!(exports.join(","), "foo");
  }

  #[test]
  fn parse_cjs_exports_case_14_1() {
    let source = r#"
      const { NODE_ENV } = process.env
      if (NODE_ENV === 'development') {
        module.exports = { foo: 'bar' }
      }
    "#;
    let swc = SWC::parse("index.cjs", source).expect("could not parse module");
    let (exports, _) = swc
      .parse_cjs_exports("development", false)
      .expect("could not parse exports");
    assert_eq!(exports.join(","), "foo");
  }

  #[test]
  fn parse_cjs_exports_case_14_2() {
    let source = r#"
      const { NODE_ENV: denv } = process.env
      if (denv === 'development') {
        module.exports = { foo: 'bar' }
      }
    "#;
    let swc = SWC::parse("index.cjs", source).expect("could not parse module");
    let (exports, _) = swc
      .parse_cjs_exports("development", false)
      .expect("could not parse exports");
    assert_eq!(exports.join(","), "foo");
  }

  #[test]
  fn parse_cjs_exports_case_14_3() {
    let source = r#"
      const denv = process.env.NODE_ENV
      if (denv === 'development') {
        module.exports = { foo: 'bar' }
      }
    "#;
    let swc = SWC::parse("index.cjs", source).expect("could not parse module");
    let (exports, _) = swc
      .parse_cjs_exports("development", false)
      .expect("could not parse exports");
    assert_eq!(exports.join(","), "foo");
  }

  #[test]
  fn parse_cjs_exports_case_14_4() {
    let source = r#"
      if (process.env.NODE_ENV !== 'development') {
        module.exports = { foo: 'bar' }
      }
    "#;
    let swc = SWC::parse("index.cjs", source).expect("could not parse module");
    let (exports, _) = swc
      .parse_cjs_exports("development", false)
      .expect("could not parse exports");
    assert_eq!(exports.join(","), "");
  }
  #[test]
  fn parse_cjs_exports_case_14_5() {
    let source = r#"
      if (typeof module !== 'undefined' && module.exports) {
        module.exports = { foo: 'bar' }
      }
    "#;
    let swc = SWC::parse("index.cjs", source).expect("could not parse module");
    let (exports, _) = swc
      .parse_cjs_exports("development", false)
      .expect("could not parse exports");
    assert_eq!(exports.join(","), "foo");
  }

  #[test]
  fn parse_cjs_exports_case_15() {
    let source = r#"
      let es = { foo: 'bar' };
      (function() {
        const { NODE_ENV } = process.env
        es.bar = 123
        if (NODE_ENV === 'development') {
          module.exports = es
        }
      })()
    "#;
    let swc = SWC::parse("index.cjs", source).expect("could not parse module");
    let (exports, _) = swc
      .parse_cjs_exports("development", false)
      .expect("could not parse exports");
    assert_eq!(exports.join(","), "foo,bar");
  }

  #[test]
  fn parse_cjs_exports_case_16() {
    let source = r#"
      function fn() { return { foo: 'bar' } };
      module.exports = fn()
    "#;
    let swc = SWC::parse("index.cjs", source).expect("could not parse module");
    let (exports, _) = swc
      .parse_cjs_exports("development", false)
      .expect("could not parse exports");
    assert_eq!(exports.join(","), "foo");
  }

  #[test]
  fn parse_cjs_exports_case_16_1() {
    let source = r#"
      let fn = () => ({ foo: 'bar' });
      module.exports = fn()
    "#;
    let swc = SWC::parse("index.cjs", source).expect("could not parse module");
    let (exports, _) = swc
      .parse_cjs_exports("development", false)
      .expect("could not parse exports");
    assert_eq!(exports.join(","), "foo");
  }

  #[test]
  fn parse_cjs_exports_case_16_2() {
    let source = r#"
      function fn() {
        const mod = { foo: 'bar' }
        mod.bar = 123
        return mod
      };
      module.exports = fn()
    "#;
    let swc = SWC::parse("index.cjs", source).expect("could not parse module");
    let (exports, _) = swc
      .parse_cjs_exports("development", false)
      .expect("could not parse exports");
    assert_eq!(exports.join(","), "foo,bar");
  }

  #[test]
  fn parse_cjs_exports_case_17() {
    let source = r#"
      module.exports = require("lib")()
    "#;
    let swc = SWC::parse("index.cjs", source).expect("could not parse module");
    let (_, reexports) = swc
      .parse_cjs_exports("development", false)
      .expect("could not parse exports");
    assert_eq!(reexports.join(","), "lib()");
  }

  #[test]
  fn parse_cjs_exports_case_18() {
    let source = r#"
      module.exports = function () {
        const mod = { foo: 'bar' }
        mod.bar = 123
        return mod
      };
    "#;
    let swc = SWC::parse("index.cjs", source).expect("could not parse module");
    let (exports, _) = swc
      .parse_cjs_exports("development", true)
      .expect("could not parse exports");
    assert_eq!(exports.join(","), "foo,bar");
  }

  #[test]
  fn parse_cjs_exports_case_18_1() {
    let source = r#"
      function fn() {
        const mod = { foo: 'bar' }
        mod.bar = 123
        return mod
      }
      module.exports = fn;
    "#;
    let swc = SWC::parse("index.cjs", source).expect("could not parse module");
    let (exports, _) = swc
      .parse_cjs_exports("development", true)
      .expect("could not parse exports");
    assert_eq!(exports.join(","), "foo,bar");
  }

  #[test]
  fn parse_cjs_exports_case_18_2() {
    let source = r#"
      const fn = () => {
        const mod = { foo: 'bar' }
        mod.bar = 123
        return mod
      }
      module.exports = fn;
    "#;
    let swc = SWC::parse("index.cjs", source).expect("could not parse module");
    let (exports, _) = swc
      .parse_cjs_exports("development", true)
      .expect("could not parse exports");
    assert_eq!(exports.join(","), "foo,bar");
  }

  #[test]
  fn parse_cjs_exports_case_18_3() {
    let source = r#"
      function fn() {
        const { NODE_ENV } = process.env
        const mod = { foo: 'bar' }
        if (NODE_ENV === 'production') {
          return mod
        }
        mod.bar = 123
        return mod
      }
      module.exports = fn;
    "#;
    let swc = SWC::parse("index.cjs", source).expect("could not parse module");
    let (exports, _) = swc
      .parse_cjs_exports("production", true)
      .expect("could not parse exports");
    assert_eq!(exports.join(","), "foo");
  }

  #[test]
  fn parse_cjs_exports_case_18_4() {
    let source = r#"
      function fn() {
        const { NODE_ENV } = process.env
        const mod = { foo: 'bar' }
        if (NODE_ENV === 'development') {
          return mod
        }
        mod.bar = 123
        return mod
      }
      module.exports = fn;
    "#;
    let swc = SWC::parse("index.cjs", source).expect("could not parse module");
    let (exports, _) = swc
      .parse_cjs_exports("production", true)
      .expect("could not parse exports");
    assert_eq!(exports.join(","), "foo,bar");
  }

  #[test]
  fn parse_cjs_exports_case_19() {
    let source = r#"
      require("tslib").__exportStar({foo: 'bar'}, exports)
      exports.bar = 123
    "#;
    let swc = SWC::parse("index.cjs", source).expect("could not parse module");
    let (exports, _) = swc
      .parse_cjs_exports("production", true)
      .expect("could not parse exports");
    assert_eq!(exports.join(","), "foo,bar");
  }

  #[test]
  fn parse_cjs_exports_case_19_2() {
    let source = r#"
      const tslib = require("tslib");
      (0, tslib.__exportStar)({foo: 'bar'}, exports)
      exports.bar = 123
    "#;
    let swc = SWC::parse("index.cjs", source).expect("could not parse module");
    let (exports, _) = swc
      .parse_cjs_exports("production", true)
      .expect("could not parse exports");
    assert_eq!(exports.join(","), "foo,bar");
  }

  #[test]
  fn parse_cjs_exports_case_19_3() {
    let source = r#"
      const { __exportStar } = require("tslib");
      (0, __exportStar)({foo: 'bar'}, exports)
      exports.bar = 123
    "#;
    let swc = SWC::parse("index.cjs", source).expect("could not parse module");
    let (exports, _) = swc
      .parse_cjs_exports("production", true)
      .expect("could not parse exports");
    assert_eq!(exports.join(","), "foo,bar");
  }

  #[test]
  fn parse_cjs_exports_case_19_4() {
    let source = r#"
      var tslib_1 = require("tslib");
      (0, tslib_1.__exportStar)(require("./crossPlatformSha256"), exports);
    "#;
    let swc = SWC::parse("index.cjs", source).expect("could not parse module");
    let (_, reexorts) = swc
      .parse_cjs_exports("production", true)
      .expect("could not parse exports");
    assert_eq!(reexorts.join(","), "./crossPlatformSha256");
  }

  #[test]
  fn parse_cjs_exports_case_19_5() {
    let source = r#"
      var __exportStar = function() {}
      Object.defineProperty(exports, "foo", { value: 1 });
      __exportStar(require("./bar"), exports);
    "#;
    let swc = SWC::parse("index.cjs", source).expect("could not parse module");
    let (exports, reexorts) = swc
      .parse_cjs_exports("production", true)
      .expect("could not parse exports");
    assert_eq!(exports.join(","), "foo");
    assert_eq!(reexorts.join(","), "./bar");
  }

  #[test]
  fn parse_cjs_exports_case_20_1() {
    let source = r#"
      var foo;
      foo = exports.foo || (exports.foo = {});
      var  bar = exports.bar || (exports.bar = {});
      exports.greet = 123;
    "#;
    let swc = SWC::parse("index.cjs", source).expect("could not parse module");
    let (exports, _) = swc
      .parse_cjs_exports("production", true)
      .expect("could not parse exports");
    assert_eq!(exports.join(","), "foo,bar,greet");
  }

  #[test]
  fn parse_cjs_exports_case_20_2() {
    let source = r#"
      var bar;
      ((foo, bar) => { })(exports.foo || (exports.foo = {}), bar = exports.bar || (exports.bar = {}));
      exports.greet = 123;
    "#;
    let swc = SWC::parse("index.cjs", source).expect("could not parse module");
    let (exports, _) = swc
      .parse_cjs_exports("production", true)
      .expect("could not parse exports");
    assert_eq!(exports.join(","), "foo,bar,greet");
  }

  #[test]
  fn parse_cjs_exports_case_21_1() {
    let source = r#"
      (function (global, factory) {
        typeof exports === 'object' && typeof module !== 'undefined' ? factory(exports) :
        typeof define === 'function' && define.amd ? define(['exports'], factory) :
        (factory((global.MMDParser = global.MMDParser || {})));
      }(this, function (exports) {
        exports.foo = "bar";
        Object.defineProperty(exports, '__esModule', { value: true });
      }))
    "#;
    let swc = SWC::parse("index.cjs", source).expect("could not parse module");
    let (exports, _) = swc
      .parse_cjs_exports("production", true)
      .expect("could not parse exports");
    assert_eq!(exports.join(","), "foo,__esModule");
  }

  #[test]
  fn parse_cjs_exports_case_21_2() {
    let source = r#"
      (function (global, factory) {
        typeof exports === 'object' && typeof module !== 'undefined' ? factory(exports) :
        typeof define === 'function' && define.amd ? define(['exports'], factory) :
        (factory((global.MMDParser = global.MMDParser || {})));
      }(this, (function (exports) {
        exports.foo = "bar";
        Object.defineProperty(exports, '__esModule', { value: true });
      })))
    "#;
    let swc = SWC::parse("index.cjs", source).expect("could not parse module");
    let (exports, _) = swc
      .parse_cjs_exports("production", true)
      .expect("could not parse exports");
    assert_eq!(exports.join(","), "foo,__esModule");
  }

  #[test]
  fn parse_cjs_exports_case_21_3() {
    // Webpack 4 minified UMD output after replacing https://github.com/glennflanagan/react-collapsible/blob/1b987617fe7c20337977a0a877574263ed7ed657/src/Collapsible.js#L1
    // with:
    // ```
    // export const named = 'named-export';

    // export default 'default-export';
    // ```
    // Manually formatted to avoid ast changes from prettier
    let source = r#"
    !function (e, t) { 
      if ("object" == typeof exports && "object" == typeof module) module.exports = t(); 
      else if ("function" == typeof define && define.amd) define([], t); 
      else { var r = t(); for (var n in r) ("object" == typeof exports ? exports : e)[n] = r[n] } 
    }(this, (function () { 
      return function (e) { 
        var t = {}; 
        function r(n) { 
          if (t[n]) return t[n].exports; 
          var o = t[n] = { i: n, l: !1, exports: {} }; 
          return e[n].call(o.exports, o, o.exports, r), o.l = !0, o.exports 
        }
        return r.m = e, 
          r.c = t, 
          r.d = function (e, t, n) { 
            r.o(e, t) || Object.defineProperty(e, t, { enumerable: !0, get: n }) 
          }, 
          r.r = function (e) { 
            "undefined" != typeof Symbol && 
            Symbol.toStringTag && 
            Object.defineProperty(e, Symbol.toStringTag, { value: "Module" }), 
            Object.defineProperty(e, "__esModule", { value: !0 }) 
          }, 
          r.t = function (e, t) { 
            if (1 & t && (e = r(e)), 8 & t) return e; 
            if (4 & t && "object" == typeof e && e && e.__esModule) return e; 
            var n = Object.create(null); 
            if (
              r.r(n), 
              Object.defineProperty(n, "default", { enumerable: !0, value: e }), 
              2 & t && 
              "string" != typeof e
            ) for (var o in e) r.d(n, o, function (t) { return e[t] }.bind(null, o)); 
            return n 
          }, 
          r.n = function (e) { 
            var t = e && e.__esModule ? 
              function () { return e.default } : 
              function () { return e };
            return r.d(t, "a", t), t 
          }, 
          r.o = function (e, t) { 
            return Object.prototype.hasOwnProperty.call(e, t)
          }, 
          r.p = "", r(r.s = 0) 
        }([
          function (e, t, r) { 
            "use strict"; 
            r.r(t), r.d(t, "named", (function () { return n })); 
            var n = "named-export"; 
            t.default = "default-export";
          }
        ]) 
      }));
    "#;
    let swc = SWC::parse("index.cjs", source).expect("could not parse module");
    let (exports, _) = swc
      .parse_cjs_exports("production", true)
      .expect("could not parse exports");
    assert_eq!(exports.join(","), "__esModule,named,default");
  }

  #[test]
  fn parse_cjs_exports_case_21_4() {
    // Webpack 5 minified UMD output after replacing https://github.com/amrlabib/react-timer-hook/blob/46aad2022d5cfa69bb24d1f4a20a94c774ea13d7/src/index.js#L1:
    // with:
    // ```
    // export const named1 = 'named-export-1';
    // export const named2 = 'named-export-2';

    // export default 'default-export';
    // ```
    // Manually formatted to avoid ast changes from prettier
    let source = r#"
    !function (e, t) {
      "object" == typeof exports && "object" == typeof module ?
        module.exports = t() :
        "function" == typeof define && define.amd ?
          define([], t) :
          "object" == typeof exports ?
            exports["react-timer-hook"] = t() :
            e["react-timer-hook"] = t()
    }(
      "undefined" != typeof self ? self : this,
      (() =>
        (() => {
          "use strict";
          var e = {
            d: (t, o) => {
              for (var r in o)
                e.o(o, r) && !e.o(t, r) && Object.defineProperty(t, r, { enumerable: !0, get: o[r] })
            },
            o: (e, t) => Object.prototype.hasOwnProperty.call(e, t),
            r: e => {
              "undefined" != typeof Symbol && Symbol.toStringTag && Object.defineProperty(e, Symbol.toStringTag, { value: "Module" }),
                Object.defineProperty(e, "__esModule", { value: !0 })
            }
          },
            t = {};
          e.r(t),
            e.d(t, {
              default: () => n,
              named1: () => o,
              named2: () => r
            });
          const o = "named-export-1",
            r = "named-export-2",
            n = "default-export";
          return t
        })()
      ));
    "#;
    let swc = SWC::parse("index.cjs", source).expect("could not parse module");
    let (exports, _) = swc
      .parse_cjs_exports("production", true)
      .expect("could not parse exports");
    assert_eq!(exports.join(","), "__esModule,default,named1,named2");
  }

  #[test]
  fn parse_cjs_exports_case_21_5() {
    // Webpack 5 minified UMD output after replacing https://github.com/amrlabib/react-timer-hook/blob/46aad2022d5cfa69bb24d1f4a20a94c774ea13d7/src/index.js#L1:
    // with:
    // ```
    // import { useEffect } from "react";

    // export default function useFn() {
    //   useEffect(() => {}, []);
    // }

    // export const named1 = "named-export-1";
    // export const named2 = "named-export-2";
    // ```
    // Formatted with Deno to avoid ast changes from prettier
    let source = r#"
    !function (e, t) {
      "object" == typeof exports && "object" == typeof module
        ? module.exports = t(require("react"))
        : "function" == typeof define && define.amd
        ? define(["react"], t)
        : "object" == typeof exports
        ? exports["react-timer-hook"] = t(require("react"))
        : e["react-timer-hook"] = t(e.react);
    }("undefined" != typeof self ? self : this, (e) =>
      (() => {
        "use strict";
        var t = {
            156: (t) => {
              t.exports = e;
            },
          },
          r = {};
        function o(e) {
          var n = r[e];
          if (void 0 !== n) return n.exports;
          var a = r[e] = { exports: {} };
          return t[e](a, a.exports, o), a.exports;
        }
        o.n = (e) => {
          var t = e && e.__esModule ? () => e.default : () => e;
          return o.d(t, { a: t }), t;
        },
          o.d = (e, t) => {
            for (var r in t) {
              o.o(t, r) && !o.o(e, r) &&
                Object.defineProperty(e, r, { enumerable: !0, get: t[r] });
            }
          },
          o.o = (e, t) => Object.prototype.hasOwnProperty.call(e, t),
          o.r = (e) => {
            "undefined" != typeof Symbol && Symbol.toStringTag &&
            Object.defineProperty(e, Symbol.toStringTag, { value: "Module" }),
              Object.defineProperty(e, "__esModule", { value: !0 });
          };
        var n = {};
        return (() => {
          o.r(n), o.d(n, { default: () => t, named1: () => r, named2: () => a });
          var e = o(156);
          function t() {
            (0, e.useEffect)(() => {}, []);
          }
          const r = "named-export-1", a = "named-export-2";
        })(),
          n;
      })());    
    "#;
    let swc = SWC::parse("index.cjs", source).expect("could not parse module");
    let (exports, _) = swc
      .parse_cjs_exports("production", true)
      .expect("could not parse exports");
    assert_eq!(exports.join(","), "__esModule,default,named1,named2");
  }

  #[test]
  fn parse_cjs_exports_case_21_6() {
    // Webpack 5 minified UMD output after replacing https://github.com/xmtp/xmtp-js-content-types/blob/c3cac4842c98785a0436240148f228c654c919c8/remote-attachment/src/index.ts
    // with:
    // ```
    // export const named1 = 'named-export-1';
    // export const named2 = 'named-export-2';

    // export default 'default-export';
    // ```
    // Formatted with Deno to avoid ast changes from prettier
    let source = r#"
    !function (e, t) {
      "object" == typeof exports && "object" == typeof module
        ? module.exports = t()
        : "function" == typeof define && define.amd
        ? define([], t)
        : "object" == typeof exports
        ? exports.xmtp = t()
        : e.xmtp = t();
    }(this, () =>
      (() => {
        "use strict";
        var e = {};
        return (() => {
          var t = e;
          Object.defineProperty(t, "__esModule", { value: !0 }),
            t.named2 = t.named1 = void 0,
            t.named1 = "named-export-1",
            t.named2 = "named-export-2",
            t.default = "default-export";
        })(),
          e;
      })());    
    "#;
    let swc = SWC::parse("index.cjs", source).expect("could not parse module");
    let (exports, _) = swc
      .parse_cjs_exports("production", true)
      .expect("could not parse exports");
    assert_eq!(exports.join(","), "__esModule,named2,named1,default");
  }

  #[test]
  fn parse_cjs_exports_case_21_7() {
    // Webpack 5 minified UMD output after replacing https://github.com/xmtp/xmtp-js-content-types/blob/c3cac4842c98785a0436240148f228c654c919c8/remote-attachment/src/index.ts
    // with:
    // ```
    // export const named1 = 'named-export-1';
    // export const named2 = 'named-export-2';

    // export default 'default-export';
    // ```
    // Formatted with Deno to avoid ast changes from prettier
    let source = r#"
    !function (e, t) {
      "object" == typeof exports && "object" == typeof module
        ? module.exports = t()
        : "function" == typeof define && define.amd
        ? define([], t)
        : "object" == typeof exports
        ? exports.xmtp = t()
        : e.xmtp = t();
    }(this, () =>
      (() => {
        "use strict";
        var e = {};
        return (() => {
          "use strict";
          var t = e;
          Object.defineProperty(t, "__esModule", { value: !0 }),
            t.named2 = t.named1 = void 0,
            t.named1 = "named-export-1",
            t.named2 = "named-export-2",
            t.default = "default-export";
        })(),
          e;
      })());    
    "#;
    let swc = SWC::parse("index.cjs", source).expect("could not parse module");
    let (exports, _) = swc
      .parse_cjs_exports("production", true)
      .expect("could not parse exports");
    assert_eq!(exports.join(","), "__esModule,named2,named1,default");
  }

  #[test]
  fn parse_cjs_exports_case_21_8() {
    // Rollup 2 minified UMD output after replacing https://github.com/Hacker0x01/react-datepicker/blob/0c13f35e11577ca0979c363a19ea2c1f34bfdf1f/src/index.jsx#L1
    // with:
    // ```
    // import { useEffect } from "react";

    // export default function useFn() {
    //   useEffect(() => {}, []);
    // }

    // export const named1 = "named-export-1";
    // export const named2 = "named-export-2";
    // ```
    // Formatted with Deno to avoid ast changes from prettier
    let source = r#"
    !function (e, t) {
      "object" == typeof exports && "undefined" != typeof module
        ? t(exports, require("react"))
        : "function" == typeof define && define.amd
        ? define(["exports", "react"], t)
        : t(
          (e = "undefined" != typeof globalThis ? globalThis : e || self)
            .DatePicker = {},
          e.React,
        );
    }(this, function (e, t) {
      "use strict";
      e.default = function () {
        t.useEffect(function () {}, []);
      },
        e.named1 = "named-export-1",
        e.named2 = "named-export-2",
        Object.defineProperty(e, "__esModule", { value: !0 });
    });
    "#;
    let swc = SWC::parse("index.cjs", source).expect("could not parse module");
    let (exports, _) = swc
      .parse_cjs_exports("production", true)
      .expect("could not parse exports");
    assert_eq!(exports.join(","), "default,named1,named2,__esModule");
  }

  #[test]
  fn parse_cjs_exports_case_21_9() {
    // Webpack 5 minified UMD output after replacing https://github.com/ttag-org/ttag/blob/622c16c8e723a15916f4c5e0fcabefe3d3ad5f84/src/index.ts#L1
    // with:
    // ```
    // function _objectSpread(target) { for (var i = 1; i < arguments.length; i++) { var source = null != arguments[i] ? arguments[i] : {}; i % 2 ? ownKeys(Object(source), !0).forEach(function (key) { _defineProperty(target, key, source[key]); }) : Object.getOwnPropertyDescriptors ? Object.defineProperties(target, Object.getOwnPropertyDescriptors(source)) : ownKeys(Object(source)).forEach(function (key) { Object.defineProperty(target, key, Object.getOwnPropertyDescriptor(source, key)); }); } return target; }
    // function _defineProperty(obj, key, value) { key = _toPropertyKey(key); if (key in obj) { Object.defineProperty(obj, key, { value: value, enumerable: true, configurable: true, writable: true }); } else { obj[key] = value; } return obj; }
    // function _toPropertyKey(arg) { var key = _toPrimitive(arg, "string"); return typeof key === "symbol" ? key : String(key); }
    // function _toPrimitive(input, hint) { if (typeof input !== "object" || input === null) return input; var prim = input[Symbol.toPrimitive]; if (prim !== undefined) { var res = prim.call(input, hint || "default"); if (typeof res !== "object") return res; throw new TypeError("@@toPrimitive must return a primitive value."); } return (hint === "string" ? String : Number)(input); }
    // export default createDedent({});
    // function createDedent() {
    //   return createDedent(_objectSpread(_objectSpread({})));
    // }

    // export const named1 = 'named1';
    // ```
    // Formatted with Deno to avoid ast changes from prettier
    let source = r#"
    !function (e, t) {
      if ("object" == typeof exports && "object" == typeof module) {
        module.exports = t();
      } else if ("function" == typeof define && define.amd) define([], t);
      else {
        var r = t();
        for (var o in r) ("object" == typeof exports ? exports : e)[o] = r[o];
      }
    }(this, () =>
      (() => {
        "use strict";
        var e = {
            d: (t, r) => {
              for (var o in r) {
                e.o(r, o) && !e.o(t, o) &&
                  Object.defineProperty(t, o, { enumerable: !0, get: r[o] });
              }
            },
            o: (e, t) => Object.prototype.hasOwnProperty.call(e, t),
            r: (e) => {
              "undefined" != typeof Symbol && Symbol.toStringTag &&
              Object.defineProperty(e, Symbol.toStringTag, { value: "Module" }),
                Object.defineProperty(e, "__esModule", { value: !0 });
            },
          },
          t = {};
        function r(e) {
          return r =
            "function" == typeof Symbol && "symbol" == typeof Symbol.iterator
              ? function (e) {
                return typeof e;
              }
              : function (e) {
                return e && "function" == typeof Symbol &&
                    e.constructor === Symbol && e !== Symbol.prototype
                  ? "symbol"
                  : typeof e;
              },
            r(e);
        }
        function o(e) {
          for (var t = 1; t < arguments.length; t++) {
            var r = null != arguments[t] ? arguments[t] : {};
            t % 2
              ? ownKeys(Object(r), !0).forEach(function (t) {
                n(e, t, r[t]);
              })
              : Object.getOwnPropertyDescriptors
              ? Object.defineProperties(e, Object.getOwnPropertyDescriptors(r))
              : ownKeys(Object(r)).forEach(function (t) {
                Object.defineProperty(e, t, Object.getOwnPropertyDescriptor(r, t));
              });
          }
          return e;
        }
        function n(e, t, o) {
          return (t = function (e) {
              var t = function (e, t) {
                if ("object" !== r(e) || null === e) return e;
                var o = e[Symbol.toPrimitive];
                if (void 0 !== o) {
                  var n = o.call(e, "string");
                  if ("object" !== r(n)) return n;
                  throw new TypeError(
                    "@@toPrimitive must return a primitive value.",
                  );
                }
                return String(e);
              }(e);
              return "symbol" === r(t) ? t : String(t);
            }(t)) in e
            ? Object.defineProperty(e, t, {
              value: o,
              enumerable: !0,
              configurable: !0,
              writable: !0,
            })
            : e[t] = o,
            e;
        }
        e.r(t), e.d(t, { default: () => i, named1: () => f });
        const i = function e() {
          return e(o(o({})));
        }();
        var f = "named1";
        return t;
      })());    
    "#;
    let swc = SWC::parse("index.cjs", source).expect("could not parse module");
    let (exports, _) = swc
      .parse_cjs_exports("production", true)
      .expect("could not parse exports");
    assert_eq!(exports.join(","), "__esModule,default,named1");
  }

  #[test]
  fn parse_cjs_exports_case_21_10() {
    // Webpack 5 minified UMD output from https://github.com/ttag-org/ttag/blob/622c16c8e723a15916f4c5e0fcabefe3d3ad5f84/src/index.ts#L1
    // with irrelevant parts manually removed.
    // This ended up being subtly different from parse_cjs_exports_case_21_9, found out when I tried testing with the full original output.
    // Formatted with Deno to avoid ast changes from prettier.
    let source = r#"
    !function (n, t) {
      if ("object" == typeof exports && "object" == typeof module) {
        module.exports = t();
      } else if ("function" == typeof define && define.amd) define([], t);
      else {
        var e = t();
        for (var r in e) ("object" == typeof exports ? exports : n)[r] = e[r];
      }
    }(this, () =>
      (() => {
        var n = {
            44: (n, t) => {
            },
            429: (n, t, e) => {
            },
          },
          t = {};
        function e(r) {
          var o = t[r];
          if (void 0 !== o) return o.exports;
          var f = t[r] = { exports: {} };
          return n[r](f, f.exports, e), f.exports;
        }
        e.d = (n, t) => {
          for (var r in t) {
            e.o(t, r) && !e.o(n, r) &&
              Object.defineProperty(n, r, { enumerable: !0, get: t[r] });
          }
        },
          e.o = (n, t) => Object.prototype.hasOwnProperty.call(n, t),
          e.r = (n) => {
            "undefined" != typeof Symbol && Symbol.toStringTag &&
            Object.defineProperty(n, Symbol.toStringTag, { value: "Module" }),
              Object.defineProperty(n, "__esModule", { value: !0 });
          };
        var r = {};
        return (() => {
          "use strict";
          function n(n, t) {
            var e = Object.keys(n);
            if (Object.getOwnPropertySymbols) {
              var r = Object.getOwnPropertySymbols(n);
              t && (r = r.filter(function (t) {
                return Object.getOwnPropertyDescriptor(n, t).enumerable;
              })), e.push.apply(e, r);
            }
            return e;
          }
          function t(t) {
            for (var e = 1; e < arguments.length; e++) {
              var r = null != arguments[e] ? arguments[e] : {};
              e % 2
                ? n(Object(r), !0).forEach(function (n) {
                  o(t, n, r[n]);
                })
                : Object.getOwnPropertyDescriptors
                ? Object.defineProperties(t, Object.getOwnPropertyDescriptors(r))
                : n(Object(r)).forEach(function (n) {
                  Object.defineProperty(
                    t,
                    n,
                    Object.getOwnPropertyDescriptor(r, n),
                  );
                });
            }
            return t;
          }
          function o(n, t, e) {
            return (t = function (n) {
                var t = function (n, t) {
                  if ("object" != typeof n || null === n) return n;
                  var e = n[Symbol.toPrimitive];
                  if (void 0 !== e) {
                    var r = e.call(n, "string");
                    if ("object" != typeof r) return r;
                    throw new TypeError(
                      "@@toPrimitive must return a primitive value.",
                    );
                  }
                  return String(n);
                }(n);
                return "symbol" == typeof t ? t : String(t);
              }(t)) in n
              ? Object.defineProperty(n, t, {
                value: e,
                enumerable: !0,
                configurable: !0,
                writable: !0,
              })
              : n[t] = e,
              n;
          }
          e.r(r),
            e.d(r, {
              Context: () => F,
              TTag: () => H,
              _: () => B,
              addLocale: () => R,
              c: () => q,
              gettext: () => U,
              jt: () => G,
              msgid: () => N,
              ngettext: () => J,
              setDedent: () => K,
              setDefaultLang: () => Q,
              t: () => V,
              useLocale: () => X,
              useLocales: () => Y,
            });
        })(),
          r;
      })());
    "#;
    let swc = SWC::parse("index.cjs", source).expect("could not parse module");
    let (exports, _) = swc
      .parse_cjs_exports("production", true)
      .expect("could not parse exports");
    assert_eq!(
      exports.join(","),
      "__esModule,Context,TTag,_,addLocale,c,gettext,jt,msgid,ngettext,setDedent,setDefaultLang,t,useLocale,useLocales"
    );
  }

  #[test]
  fn parse_cjs_exports_case_21_11() {
    // Webpack 5 minified UMD output from https://github.com/OfficeDev/microsoft-teams-library-js/blob/b5ee6475a7ed333cdb27f25fc51e5bb1ebd0da94/packages/teams-js/src/index.ts#L1
    // with irrelevant parts manually removed.
    // with umdNamedDefine: true: https://webpack.js.org/configuration/output/#outputlibraryumdnameddefine
    // Formatted with Deno to avoid ast changes from prettier.
    let source = r#"
    !function (e, t) {
      "object" == typeof exports && "object" == typeof module
        ? module.exports = t()
        : "function" == typeof define && define.amd
        ? define("microsoftTeams", [], t)
        : "object" == typeof exports
        ? exports.microsoftTeams = t()
        : e.microsoftTeams = t();
    }("undefined" != typeof self ? self : this, () =>
      (() => {
        var e = {
            302: (e, t, n) => {
            },
            65: (e, t, n) => {},
            247: (e) => {},
          },
          t = {};
        function n(o) {
          var i = t[o];
          if (void 0 !== i) return i.exports;
          var r = t[o] = { exports: {} };
          return e[o](r, r.exports, n), r.exports;
        }
        (() => {
          var e,
            t = Object.getPrototypeOf
              ? (e) => Object.getPrototypeOf(e)
              : (e) => e.__proto__;
          n.t = function (o, i) {
            if (1 & i && (o = this(o)), 8 & i) return o;
            if ("object" == typeof o && o) {
              if (4 & i && o.__esModule) return o;
              if (16 & i && "function" == typeof o.then) return o;
            }
            var r = Object.create(null);
            n.r(r);
            var a = {};
            e = e || [null, t({}), t([]), t(t)];
            for (
              var s = 2 & i && o; "object" == typeof s && !~e.indexOf(s); s = t(s)
            ) Object.getOwnPropertyNames(s).forEach((e) => a[e] = () => o[e]);
            return a.default = () => o, n.d(r, a), r;
          };
        })(),
          (() => {
            n.d = (e, t) => {
              for (var o in t) {n.o(t, o) && !n.o(e, o) &&
                  Object.defineProperty(e, o, { enumerable: !0, get: t[o] });}
            };
          })(),
          (() => {
            n.o = (e, t) => Object.prototype.hasOwnProperty.call(e, t);
          })(),
          (() => {
            n.r = (e) => {
              "undefined" != typeof Symbol && Symbol.toStringTag &&
              Object.defineProperty(e, Symbol.toStringTag, { value: "Module" }),
                Object.defineProperty(e, "__esModule", { value: !0 });
            };
          })();
        var o = {};
        return (() => {
          "use strict";
          n.r(o),
            n.d(o, {
              app: () => pt,
            });
        })(),
          o;
      })());    
    "#;
    let swc = SWC::parse("index.cjs", source).expect("could not parse module");
    let (exports, _) = swc
      .parse_cjs_exports("production", true)
      .expect("could not parse exports");
    assert_eq!(exports.join(","), "__esModule,app");
  }

  #[test]
  fn parse_cjs_exports_case_21_12() {
    // Webpack 3 minified UMD output from https://github.com/highcharts/highcharts-react/blob/08e5ac19b9da5ba7d5b463919691e0b1db0fb34c/dist/highcharts-react.min.js#L1C1-L2C49
    // with irrelevant parts manually removed.
    // Formatted with Deno to avoid ast changes from prettier.
    let source = r#"
    !function (e, t) {
      "object" == typeof exports && "object" == typeof module
        ? module.exports = t(require("react"))
        : "function" == typeof define && define.amd
        ? define(["react"], t)
        : "object" == typeof exports
        ? exports.HighchartsReact = t(require("react"))
        : e.HighchartsReact = t(e.React);
    }("undefined" != typeof self ? self : this, function (e) {
      return function (e) {
        function t(n) {
          if (r[n]) return r[n].exports;
          var o = r[n] = { i: n, l: !1, exports: {} };
          return e[n].call(o.exports, o, o.exports, t), o.l = !0, o.exports;
        }
        var r = {};
        return t.m = e,
          t.c = r,
          t.d = function (e, r, n) {
            t.o(e, r) ||
              Object.defineProperty(e, r, {
                configurable: !1,
                enumerable: !0,
                get: n,
              });
          },
          t.n = function (e) {
            var r = e && e.__esModule
              ? function () {
                return e.default;
              }
              : function () {
                return e;
              };
            return t.d(r, "a", r), r;
          },
          t.o = function (e, t) {
            return Object.prototype.hasOwnProperty.call(e, t);
          },
          t.p = "",
          t(t.s = 0);
      }([function (e, t, r) {
        "use strict";
        Object.defineProperty(t, "__esModule", { value: !0 }),
          r.d(t, "HighchartsReact", function () {
            return c;
          });
        var n = r(1),
          o = r.n(n),
          c = function (e) {
            return o.a.createElement("div", e.containerProps);
          };
        t.default = c;
      }, function (t, r) {
        t.exports = e;
      }]);
    });
    "#;
    let swc = SWC::parse("index.cjs", source).expect("could not parse module");
    let (exports, _) = swc
      .parse_cjs_exports("production", true)
      .expect("could not parse exports");
    assert_eq!(exports.join(","), "HighchartsReact,default");
  }

  #[test]
  fn parse_cjs_exports_case_22() {
    let source = r#"
      var url = module.exports = {};
      url.foo = 'bar';
    "#;
    let swc = SWC::parse("index.cjs", source).expect("could not parse module");
    let (exports, _) = swc
      .parse_cjs_exports("production", true)
      .expect("could not parse exports");
    assert_eq!(exports.join(","), "foo");
  }

  #[test]
  fn parse_cjs_exports_case_22_2() {
    let source = r#"
      exports.i18n = exports.use = exports.t = undefined;
    "#;
    let swc = SWC::parse("index.cjs", source).expect("could not parse module");
    let (exports, _) = swc
      .parse_cjs_exports("production", true)
      .expect("could not parse exports");
    assert_eq!(exports.join(","), "i18n,use,t");
  }

  #[test]
  fn parse_cjs_exports_case_23() {
    let source = r#"
      Object.defineProperty(exports, "__esModule", { value: true });
      __export({foo:"bar"});
      __export(require("./lib"));
    "#;
    let swc = SWC::parse("index.cjs", source).expect("could not parse module");
    let (exports, reexports) = swc
      .parse_cjs_exports("production", true)
      .expect("could not parse exports");
    assert_eq!(exports.join(","), "__esModule,foo");
    assert_eq!(reexports.join(","), "./lib");
  }

  #[test]
  fn parse_cjs_exports_case_24() {
    let source = r#"
    0 && (module.exports = {
      foo,
      bar
    });
    "#;
    let swc = SWC::parse("index.cjs", source).expect("could not parse module");
    let (exports, _) = swc
      .parse_cjs_exports("production", true)
      .expect("could not parse exports");
    assert_eq!(exports.join(","), "foo,bar");
  }
}
