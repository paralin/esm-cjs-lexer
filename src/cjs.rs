use indexmap::{IndexMap, IndexSet};
use std::vec;
use swc_common::DUMMY_SP;
use swc_ecma_ast::*;
use swc_ecma_visit::{noop_fold_type, Fold};

#[derive(Clone, Debug)]
pub enum IdentKind {
  Lit(Lit),
  Alias(String),
  Object(Vec<PropOrSpread>),
  Class(Class),
  Fn(FnDesc),
  Reexport(String),
  Unkonwn,
}

#[derive(Clone, Debug)]
pub struct FnDesc {
  stmts: Vec<Stmt>,
  extends: Vec<String>,
}

pub struct CJSLexer {
  pub node_env: String,
  pub call_mode: bool,
  pub fn_returned: bool,
  pub exports_alias: IndexSet<String>,
  pub idents: IndexMap<String, IdentKind>,
  pub exports: IndexSet<String>,
  pub reexports: IndexSet<String>,
}

impl CJSLexer {
  fn clear(&mut self) {
    self.exports.clear();
    self.reexports.clear();
  }

  fn reset(&mut self, expr: &Expr) {
    if let Expr::Paren(ParenExpr { expr, .. }) = expr {
      self.reset(expr);
      return;
    }
    if let Some(reexport) = self.as_reexport(expr) {
      self.clear();
      self.reexports.insert(reexport);
    } else if let Some(props) = self.as_obj(expr) {
      self.clear();
      self.use_object_as_exports(props);
    } else if let Some(class) = self.as_class(expr) {
      self.clear();
      for name in get_class_static_names(&class) {
        self.exports.insert(name);
      }
    } else if let Some(FnDesc { stmts, extends }) = self.as_function(expr) {
      self.clear();
      if self.call_mode {
        self.walk_body(stmts, true);
      } else {
        for name in extends {
          self.exports.insert(name);
        }
      }
    } else if let Expr::Call(call) = expr {
      if call.args.len() == 0 {
        if let Some(callee) = with_expr_callee(call) {
          if let Some(reexport) = self.as_reexport(callee) {
            self.clear();
            self.reexports.insert(format!("{}()", reexport));
          } else if let Some(FnDesc { stmts, .. }) = self.as_function(callee) {
            self.walk_body(stmts, true);
          }
        }
      }
    }
  }

  fn mark_ident(&mut self, name: &str, expr: &Expr) {
    if let Expr::Paren(ParenExpr { expr, .. }) = expr {
      self.mark_ident(name, expr);
      return;
    }
    match expr {
      Expr::Lit(lit) => {
        self.idents.insert(name.into(), IdentKind::Lit(lit.clone()));
      }
      Expr::Ident(id) => {
        let conflict = if let Some(val) = self.idents.get(id.sym.as_ref()) {
          if let IdentKind::Alias(rename) = val {
            rename.eq(name)
          } else {
            false
          }
        } else {
          false
        };
        if !conflict {
          self
            .idents
            .insert(name.into(), IdentKind::Alias(id.sym.as_ref().into()));
        }
      }
      Expr::Call(call) => {
        if let Some(file) = is_require_call(&call) {
          self.idents.insert(name.into(), IdentKind::Reexport(file));
        }
      }
      Expr::Object(obj) => {
        self.idents.insert(name.into(), IdentKind::Object(obj.props.clone()));
      }
      Expr::Class(ClassExpr { class, .. }) => {
        self
          .idents
          .insert(name.into(), IdentKind::Class(class.as_ref().clone()));
      }
      Expr::Arrow(arrow) => {
        self.idents.insert(
          name.into(),
          IdentKind::Fn(FnDesc {
            stmts: get_arrow_body_as_stmts(&arrow),
            extends: vec![],
          }),
        );
      }
      Expr::Fn(FnExpr { function, .. }) => {
        if let Function { body: Some(body), .. } = function.as_ref() {
          self.idents.insert(
            name.into(),
            IdentKind::Fn(FnDesc {
              stmts: body.stmts.clone(),
              extends: vec![],
            }),
          );
        };
      }
      Expr::Member(_) => {
        if is_member_member(expr, "process", "env", "NODE_ENV") {
          self
            .idents
            .insert(name.into(), IdentKind::Lit(Lit::Str(quote_str(self.node_env.as_str()))));
        }
      }
      _ => {
        self.idents.insert(name.into(), IdentKind::Unkonwn);
      }
    };
  }

  fn as_str(&self, expr: &Expr) -> Option<String> {
    match expr {
      Expr::Paren(ParenExpr { expr, .. }) => return self.as_str(expr),
      Expr::Lit(Lit::Str(Str { value, .. })) => return Some(value.as_ref().into()),
      Expr::Ident(id) => {
        if let Some(value) = self.idents.get(id.sym.as_ref()) {
          match value {
            IdentKind::Lit(Lit::Str(Str { value, .. })) => return Some(value.as_ref().into()),
            IdentKind::Alias(id) => return self.as_str(&Expr::Ident(quote_ident(id))),
            _ => {}
          }
        }
      }
      Expr::Member(_) => {
        if is_member_member(expr, "process", "env", "NODE_ENV") {
          return Some(self.node_env.to_owned());
        }
      }
      _ => {}
    };
    None
  }

  fn as_num(&self, expr: &Expr) -> Option<f64> {
    match expr {
      Expr::Paren(ParenExpr { expr, .. }) => return self.as_num(expr),
      Expr::Lit(Lit::Num(Number { value, .. })) => return Some(*value),
      Expr::Ident(id) => {
        if let Some(value) = self.idents.get(id.sym.as_ref()) {
          match value {
            IdentKind::Lit(Lit::Num(Number { value, .. })) => return Some(*value),
            IdentKind::Alias(id) => return self.as_num(&Expr::Ident(quote_ident(id))),
            _ => {}
          }
        }
      }
      _ => {}
    };
    None
  }

  fn as_bool(&self, expr: &Expr) -> Option<bool> {
    match expr {
      Expr::Paren(ParenExpr { expr, .. }) => return self.as_bool(expr),
      Expr::Lit(Lit::Bool(Bool { value, .. })) => return Some(*value),
      Expr::Ident(id) => {
        if let Some(value) = self.idents.get(id.sym.as_ref()) {
          match value {
            IdentKind::Lit(Lit::Bool(Bool { value, .. })) => return Some(*value),
            IdentKind::Alias(id) => return self.as_bool(&Expr::Ident(quote_ident(id))),
            _ => {}
          }
        }
      }
      _ => {}
    };
    None
  }

  fn as_null(&self, expr: &Expr) -> Option<bool> {
    match expr {
      Expr::Paren(ParenExpr { expr, .. }) => return self.as_null(expr),
      Expr::Lit(Lit::Null(_)) => return Some(true),
      Expr::Ident(id) => {
        if let Some(value) = self.idents.get(id.sym.as_ref()) {
          match value {
            IdentKind::Lit(Lit::Null(_)) => return Some(true),
            IdentKind::Alias(id) => return self.as_null(&Expr::Ident(quote_ident(id))),
            _ => {}
          }
        }
      }
      _ => {}
    };
    None
  }

  fn as_obj(&self, expr: &Expr) -> Option<Vec<PropOrSpread>> {
    match expr {
      Expr::Paren(ParenExpr { expr, .. }) => return self.as_obj(expr),
      Expr::Object(ObjectLit { props, .. }) => Some(props.to_vec()),
      Expr::Ident(id) => {
        if let Some(value) = self.idents.get(id.sym.as_ref()) {
          match value {
            IdentKind::Object(props) => return Some(props.to_vec()),
            IdentKind::Alias(id) => return self.as_obj(&Expr::Ident(quote_ident(id))),
            _ => {}
          }
        }
        None
      }
      _ => None,
    }
  }

  fn as_reexport(&self, expr: &Expr) -> Option<String> {
    match expr {
      Expr::Paren(ParenExpr { expr, .. }) => return self.as_reexport(expr),
      Expr::Call(call) => is_require_call(&call),
      Expr::Ident(id) => {
        if let Some(value) = self.idents.get(id.sym.as_ref()) {
          match value {
            IdentKind::Reexport(file) => return Some(file.to_owned()),
            IdentKind::Alias(id) => return self.as_reexport(&Expr::Ident(quote_ident(id))),
            _ => {}
          }
        }
        None
      }
      _ => None,
    }
  }

  fn as_class(&self, expr: &Expr) -> Option<Class> {
    match expr {
      Expr::Paren(ParenExpr { expr, .. }) => return self.as_class(expr),
      Expr::Class(ClassExpr { class, .. }) => Some(class.as_ref().clone()),
      Expr::Ident(id) => {
        if let Some(value) = self.idents.get(id.sym.as_ref()) {
          match value {
            IdentKind::Class(class) => return Some(class.clone()),
            IdentKind::Alias(id) => return self.as_class(&Expr::Ident(quote_ident(id))),
            _ => {}
          }
        }
        None
      }
      _ => None,
    }
  }

  fn as_function(&self, expr: &Expr) -> Option<FnDesc> {
    match expr {
      Expr::Paren(ParenExpr { expr, .. }) => return self.as_function(expr),
      Expr::Fn(FnExpr { function, .. }) => {
        if let Function { body: Some(body), .. } = function.as_ref() {
          Some(FnDesc {
            stmts: body.stmts.clone(),
            extends: vec![],
          })
        } else {
          None
        }
      }
      Expr::Ident(id) => {
        if let Some(value) = self.idents.get(id.sym.as_ref()) {
          match value {
            IdentKind::Fn(desc) => return Some(desc.clone()),
            IdentKind::Alias(id) => return self.as_function(&Expr::Ident(quote_ident(id))),
            _ => {}
          }
        }
        None
      }
      _ => None,
    }
  }

  fn use_object_as_exports(&mut self, props: Vec<PropOrSpread>) {
    for prop in props {
      match prop {
        PropOrSpread::Prop(prop) => {
          let name = match prop.as_ref() {
            Prop::Shorthand(id) => Some(id.sym.as_ref().to_owned()),
            Prop::KeyValue(KeyValueProp { key, .. }) => stringify_prop_name(key),
            Prop::Method(MethodProp { key, .. }) => stringify_prop_name(key),
            _ => None,
          };
          if let Some(name) = name {
            self.exports.insert(name);
          }
        }
        PropOrSpread::Spread(SpreadElement { expr, .. }) => match expr.as_ref() {
          Expr::Ident(_) => {
            if let Some(props) = self.as_obj(expr.as_ref()) {
              self.use_object_as_exports(props);
            }
            if let Some(reexport) = self.as_reexport(expr.as_ref()) {
              self.reexports.insert(reexport);
            }
          }
          Expr::Call(call) => {
            if let Some(reexport) = is_require_call(call) {
              self.reexports.insert(reexport);
            }
          }
          _ => {}
        },
      }
    }
  }

  fn eqeq(&self, left: &Expr, right: &Expr) -> bool {
    if let Some(left) = self.as_str(left) {
      if let Some(right) = self.as_str(right) {
        return left == right;
      }
    } else if let Some(left) = self.as_num(left) {
      if let Some(right) = self.as_num(right) {
        return left == right;
      }
    } else if let Some(left) = self.as_bool(left) {
      if let Some(right) = self.as_bool(right) {
        return left == right;
      }
    } else if let Some(left) = self.as_null(left) {
      if let Some(right) = self.as_null(right) {
        return left == right;
      }
    }
    false
  }

  fn is_true(&self, expr: &Expr) -> bool {
    match expr {
      Expr::Paren(ParenExpr { expr, .. }) => return self.is_true(expr),
      Expr::Ident(id) => {
        if let Some(value) = self.idents.get(id.sym.as_ref()) {
          match value {
            IdentKind::Lit(lit) => return self.is_true(&Expr::Lit(lit.clone())),
            IdentKind::Alias(id) => return self.is_true(&Expr::Ident(quote_ident(id))),
            _ => {}
          }
        } else {
          return false; // undefined
        }
      }
      Expr::Lit(lit) => {
        return match lit {
          Lit::Bool(Bool { value, .. }) => *value,
          Lit::Str(Str { value, .. }) => !value.as_ref().is_empty(),
          Lit::Null(_) => false,
          Lit::Num(Number { value, .. }) => *value != 0.0,
          _ => false,
        }
      }
      Expr::Bin(BinExpr { op, left, right, .. }) => {
        if matches!(op, BinaryOp::LogicalAnd) {
          return self.is_true(left) && self.is_true(right);
        }
        if matches!(op, BinaryOp::LogicalOr) {
          return self.is_true(left) || self.is_true(right);
        }
        if matches!(op, BinaryOp::EqEq | BinaryOp::EqEqEq) {
          return self.eqeq(left, right);
        }
        if matches!(op, BinaryOp::NotEq | BinaryOp::NotEqEq) {
          return !self.eqeq(left, right);
        }
      }
      _ => {}
    }
    true
  }

  // var foo = module.exports = {};
  // foo === module.exports;
  fn try_to_mark_exports_alias(&mut self, decl: &VarDeclarator) {
    if let Pat::Ident(id) = &decl.name {
      if let Some(init) = &decl.init {
        if is_member(init, "module", "exports") {
          self.exports_alias.insert(id.id.sym.as_ref().to_owned());
        } else if let Expr::Assign(assign) = init.as_ref() {
          if let Some(member) = get_member_expr_from_assign_target(&assign.left) {
            if is_member(&Expr::Member(member.clone()), "module", "exports") {
              self.exports_alias.insert(id.id.sym.as_ref().to_owned());
            }
          }
        }
      }
    }
  }

  fn is_exports_ident(&self, id: &str) -> bool {
    return id.eq("exports") || self.exports_alias.contains(id);
  }

  fn is_exports_expr(&self, expr: &Expr) -> bool {
    match expr {
      Expr::Ident(id) => {
        let id = id.sym.as_ref();
        self.is_exports_ident(id)
      }
      Expr::Member(_) => is_member(expr, "module", "exports"),
      _ => false,
    }
  }

  fn get_exports_prop_name(&self, expr: &Expr) -> Option<String> {
    if let Expr::Member(MemberExpr { obj, prop, .. }) = expr {
      if let Expr::Ident(obj) = obj.as_ref() {
        if self.is_exports_ident(obj.sym.as_ref()) {
          return get_prop_name(prop);
        }
      }
    }
    None
  }

  // exports.foo || (exports.foo = {})
  // foo = exports.foo || (exports.foo = {})
  fn get_bare_export_names(&mut self, expr: &Expr) -> Option<String> {
    if let Expr::Assign(assign) = expr {
      return self.get_bare_export_names(assign.right.as_ref());
    }

    if let Expr::Bin(bin) = expr {
      if bin.op == BinaryOp::LogicalOr {
        if let Some(member_prop_name) = self.get_exports_prop_name(bin.left.as_ref()) {
          if let Expr::Paren(ParenExpr { expr, .. }) = bin.right.as_ref() {
            if let Expr::Assign(assign) = expr.as_ref() {
              if let AssignTarget::Simple(expr) = &assign.left {
                if let SimpleAssignTarget::Member(MemberExpr { obj, prop, .. }) = expr {
                  if let Expr::Ident(obj) = obj.as_ref() {
                    if self.is_exports_ident(obj.sym.as_ref()) {
                      if let Some(prop_name) = get_prop_name(prop) {
                        if prop_name.eq(&member_prop_name) {
                          return Some(member_prop_name);
                        }
                      }
                    }
                  }
                }
              }
            }
          }
        }
      }
    }

    None
  }

  fn get_exports_from_assign(&mut self, assign: &AssignExpr) {
    if assign.op == AssignOp::Assign {
      let member = if let AssignTarget::Simple(simple) = &assign.left {
        if let SimpleAssignTarget::Member(member) = &simple {
          Some(member)
        } else {
          None
        }
      } else {
        None
      };
      if let Some(MemberExpr { obj, prop, .. }) = member {
        let prop = get_prop_name(&prop);
        if let Some(prop) = prop {
          match obj.as_ref() {
            Expr::Ident(obj) => {
              let obj_name = obj.sym.as_ref();
              if self.is_exports_ident(obj_name) {
                // exports.foo = 'bar'
                self.exports.insert(prop);
                if let Expr::Assign(dep_assign) = assign.right.as_ref() {
                  self.get_exports_from_assign(dep_assign);
                }
              } else if obj_name.eq("module") && self.is_exports_ident(&prop) {
                // module.exports = ??
                let right_expr = assign.right.as_ref();
                self.reset(right_expr)
              }
            }
            Expr::Member(_) => {
              if is_member(obj, "module", "exports") {
                self.exports.insert(prop);
              }
            }
            _ => {}
          }
        }
      } else {
        if let Some(bare_export_name) = self.get_bare_export_names(assign.right.as_ref()) {
          self.exports.insert(bare_export_name);
        }
      }
    }
  }

  // function (e, t, r) {
  //   "use strict";
  //   r.r(t), r.d(t, "named", (function () { return n }));
  //   var n = "named-export";
  //   t.default = "default-export";
  // }
  fn get_webpack4_exports(&mut self, expr: &Expr, webpack_exports_sym: &str, webpack_require_sym: Option<&str>) {
    match &*expr {
      Expr::Seq(SeqExpr { exprs, .. }) => {
        for expr in exprs {
          self.get_webpack4_exports(&expr, webpack_exports_sym, webpack_require_sym)
        }
      }
      Expr::Call(call) => match webpack_require_sym {
        Some(webpack_require_sym) => {
          if let Some(Expr::Member(MemberExpr { obj, prop, .. })) = with_expr_callee(call) {
            if let (Expr::Ident(Ident { sym: obj_sym, .. }), MemberProp::Ident(Ident { sym: prop_sym, .. })) =
              (&**obj, &*prop)
            {
              if obj_sym.as_ref().eq(webpack_require_sym) && prop_sym.as_ref().eq("r") {
                self.exports.insert("__esModule".to_string());
              }
              if obj_sym.as_ref().eq(webpack_require_sym) && prop_sym.as_ref().eq("d") {
                let CallExpr { args, .. } = &*call;
                if let Some(ExprOrSpread { expr, .. }) = args.get(1) {
                  if let Expr::Lit(Lit::Str(Str { value, .. })) = &**expr {
                    self.exports.insert(value.as_ref().to_string());
                  }
                }
              }
            }
          }
        }
        None => {}
      },
      Expr::Assign(AssignExpr {
        left,
        op: AssignOp::Assign,
        ..
      }) => {
        // if let PatOrExpr::Expr(expr) = &*left {
        //   if let Expr::Member(MemberExpr { obj, prop, .. }) = &**expr {
        //     if let Expr::Ident(Ident { sym, .. }) = &**obj {
        //       if sym.as_ref().eq(webpack_exports_sym.as_ref()) {
        //         if let MemberProp::Ident(prop) = prop {
        //           if prop.sym.as_ref().eq("default") {
        //             self.exports.insert("default".to_string());
        //           }
        //         }
        //       }
        //     }
        //   }
        // }
        // This doesn't feel right but is what ends up matching
        // t.default = "default-export"
        // May be an swc ast bug
        if let AssignTarget::Simple(simple) = &left {
          if let SimpleAssignTarget::Member(MemberExpr { obj, prop, .. }) = &simple {
            if let Expr::Ident(Ident { sym, .. }) = &**obj {
              if sym.as_ref().eq(webpack_exports_sym) {
                if let MemberProp::Ident(prop) = prop {
                  if prop.sym.as_ref().eq("default") {
                    self.exports.insert("default".to_string());
                  }
                }
              }
            }
          }
        }
      }
      _ => {}
    }
  }

  fn get_webpack_exports(&mut self, stmts: &Vec<Stmt>, webpack_require_sym: &str, first_stmt_index: &usize) {
    stmts.iter().skip(*first_stmt_index).take(8).find(|stmt| match stmt {
      Stmt::Expr(ExprStmt { expr, .. }) => {
        if let Expr::Seq(SeqExpr { exprs, .. }) = &**expr {
          let mut found_webpack_require_exprs = false;
          for expr in exprs {
            if let Expr::Call(call) = &**expr {
              if let Some(Expr::Member(MemberExpr { obj, prop, .. })) = with_expr_callee(call) {
                if let (Expr::Ident(Ident { sym: obj_sym, .. }), MemberProp::Ident(Ident { sym: prop_sym, .. })) =
                  (&**obj, &*prop)
                {
                  if !obj_sym.as_ref().eq(webpack_require_sym) {
                    return false;
                  }
                  let prop_sym_ref = prop_sym.as_ref();

                  if prop_sym_ref.eq("r") {
                    self.exports.insert("__esModule".to_string());
                    found_webpack_require_exprs = true;
                  }
                  if prop_sym_ref.eq("d") {
                    let CallExpr { args, .. } = &*call;
                    if let Some(ExprOrSpread { expr, .. }) = args.get(1) {
                      if let Expr::Object(ObjectLit { props, .. }) = &**expr {
                        for prop in props {
                          if let PropOrSpread::Prop(prop) = prop {
                            if let Prop::KeyValue(KeyValueProp {
                              key: PropName::Ident(Ident { sym, .. }),
                              ..
                            }) = &**prop
                            {
                              self.exports.insert(sym.as_ref().to_string());
                              found_webpack_require_exprs = true;
                            }
                          }
                        }
                      }
                    }
                  }
                }
              }
            }
          }
          return found_webpack_require_exprs;
        }
        return false;
      }
      _ => false,
    });
  }

  fn get_webpack_require_props_from_props(&mut self, props: &Vec<PropOrSpread>) -> i32 {
    props
      .iter()
      .map(|prop| match prop {
        PropOrSpread::Prop(prop) => match &**prop {
          Prop::KeyValue(KeyValueProp {
            key: PropName::Ident(Ident { sym, .. }),
            ..
          }) => {
            let sym_ref = sym.as_ref();
            if sym_ref.eq("r") || sym_ref.eq("d") {
              return 1;
            }
            return 0;
          }
          _ => 0,
        },
        _ => 0,
      })
      .sum()
  }

  fn get_webpack_require_props_from_stmts(&mut self, stmts: &Vec<Stmt>, webpack_require_sym: &str) -> i32 {
    return stmts
      .iter()
      .map(|stmt| {
        if let Stmt::Expr(ExprStmt { expr, .. }) = stmt {
          match &**expr {
            Expr::Seq(SeqExpr { exprs, .. }) => {
              return exprs
                .iter()
                .map(|expr| match &**expr {
                  Expr::Assign(AssignExpr {
                    op: AssignOp::Assign,
                    left,
                    ..
                  }) => {
                    if let AssignTarget::Simple(simple) = &left {
                      if let SimpleAssignTarget::Member(MemberExpr {
                        obj,
                        prop: MemberProp::Ident(Ident { sym: prop_sym, .. }),
                        ..
                      }) = &simple
                      {
                        if let Expr::Ident(Ident { sym, .. }) = &**obj {
                          if sym.as_ref().eq(<str as AsRef<str>>::as_ref(webpack_require_sym)) {
                            let prop_sym_ref = prop_sym.as_ref();
                            if prop_sym_ref.eq("r") || prop_sym_ref.eq("d") {
                              return 1;
                            }
                          }
                        }
                      }
                    }
                    return 0;
                  }
                  Expr::Call(call) => {
                    if let Some(body) = is_iife_call(call) {
                      return self.get_webpack_require_props_from_stmts(&body, webpack_require_sym);
                    }
                    return 0;
                  }
                  _ => 0,
                })
                .sum();
            }
            Expr::Assign(AssignExpr {
              op: AssignOp::Assign,
              left,
              ..
            }) => {
              if let AssignTarget::Simple(simple) = &left {
                if let SimpleAssignTarget::Member(MemberExpr {
                  obj,
                  prop: MemberProp::Ident(Ident { sym: prop_sym, .. }),
                  ..
                }) = &simple
                {
                  if let Expr::Ident(Ident { sym, .. }) = &**obj {
                    if sym.as_ref().eq(<str as AsRef<str>>::as_ref(webpack_require_sym)) {
                      let prop_sym_ref = prop_sym.as_ref();
                      if prop_sym_ref.eq("r") || prop_sym_ref.eq("d") {
                        return 1;
                      }
                    }
                  }
                }
              }
              return 0;
            }
            _ => 0,
          };
        }
        return 0;
      })
      .sum();
  }
  fn is_umd_iife_call(&mut self, call: &CallExpr) -> Option<Vec<Stmt>> {
    if call.args.len() == 2 {
      let mut arg1 = call.args.get(1).unwrap().expr.as_ref();
      if let Expr::Paren(ParenExpr { expr, .. }) = arg1 {
        arg1 = expr.as_ref();
      }
      let stmts = match arg1 {
        Expr::Fn(func) => {
          if let Some(BlockStmt { stmts, .. }) = &func.function.body {
            Some(stmts.clone())
          } else {
            None
          }
        }
        Expr::Arrow(arrow) => Some(get_arrow_body_as_stmts(&arrow)),
        _ => None,
      };
      let expr = if let Some(callee) = with_expr_callee(call) {
        match callee {
          Expr::Paren(ParenExpr { expr, .. }) => expr.as_ref(),
          _ => callee,
        }
      } else {
        return None;
      };
      match expr {
        Expr::Fn(func) => {
          if is_umd_params(&func.function.params.iter().map(|p| p.pat.clone()).collect()) {
            return stmts;
          } else if let Some(BlockStmt { stmts: body_stmts, .. }) = &func.function.body {
            if is_umd_checks(body_stmts) {
              if let Some(Param {
                pat: Pat::Ident(BindingIdent {
                  id: Ident { sym, .. }, ..
                }),
                ..
              }) = &func.function.params.get(0)
              {
                self.exports_alias.insert(sym.as_ref().to_owned());
              }

              return stmts;
            }
            return None;
          }
        }
        Expr::Arrow(arrow) => {
          // TODO: detect for minified umd, haven't seen any in the wild using arrow fns yet though
          if is_umd_params(&arrow.params) {
            return stmts;
          }
        }
        _ => {}
      }
    }
    None
  }

  // walk and mark idents
  fn walk_stmts(&mut self, stmts: &Vec<Stmt>) -> bool {
    for stmt in stmts {
      match stmt {
        Stmt::Decl(decl) => match decl {
          Decl::Var(var) => {
            for decl in &var.decls {
              self.try_to_mark_exports_alias(decl);
              match &decl.name {
                Pat::Ident(BindingIdent { id, .. }) => {
                  let id = id.sym.as_ref();
                  if let Some(init) = &decl.init {
                    self.mark_ident(id, init);
                  } else {
                    self.idents.insert(id.into(), IdentKind::Unkonwn);
                  }
                }
                Pat::Object(ObjectPat { props, .. }) => {
                  let mut process_env_init = false;
                  if let Some(init) = &decl.init {
                    process_env_init = is_member(init.as_ref(), "process", "env");
                  };
                  if process_env_init {
                    for prop in props {
                      match prop {
                        ObjectPatProp::Assign(AssignPatProp { key, .. }) => {
                          let key = key.sym.as_ref();
                          if key.eq("NODE_ENV") {
                            self.idents.insert(
                              key.to_owned(),
                              IdentKind::Lit(Lit::Str(quote_str(self.node_env.as_str()))),
                            );
                          }
                        }
                        ObjectPatProp::KeyValue(KeyValuePatProp { key, value, .. }) => {
                          let key = stringify_prop_name(&key);
                          if let (Some(key), Pat::Ident(rename)) = (key, value.as_ref()) {
                            if key.eq("NODE_ENV") {
                              self.idents.insert(
                                rename.id.sym.as_ref().to_owned(),
                                IdentKind::Lit(Lit::Str(quote_str(self.node_env.as_str()))),
                              );
                            }
                          }
                        }
                        _ => {}
                      }
                    }
                  }
                }
                _ => {}
              }
            }
          }
          Decl::Fn(FnDecl { ident, function, .. }) => {
            self.mark_ident(
              ident.sym.as_ref(),
              &Expr::Fn(FnExpr {
                ident: Some(ident.clone()),
                function: function.clone(),
              }),
            );
          }
          Decl::Class(ClassDecl { ident, class, .. }) => {
            self.mark_ident(
              ident.sym.as_ref(),
              &Expr::Class(ClassExpr {
                ident: Some(ident.clone()),
                class: class.clone(),
              }),
            );
          }
          _ => {}
        },
        Stmt::Expr(ExprStmt { expr, .. }) => {
          match expr.as_ref() {
            Expr::Assign(assign) => {
              if assign.op == AssignOp::Assign {
                match &assign.left {
                  AssignTarget::Simple(simple) => match simple {
                    // var foo = 'boo'
                    // foo = 'bar'
                    SimpleAssignTarget::Ident(BindingIdent { id, .. }) => {
                      let id = id.sym.as_ref();
                      if self.idents.contains_key(id) {
                        self.mark_ident(id, &assign.right.as_ref())
                      }
                    }
                    // var foo = {}
                    // foo.bar = 'bar'
                    SimpleAssignTarget::Member(MemberExpr { obj, prop, .. }) => {
                      let key = get_prop_name(&prop);
                      if let Some(key) = key {
                        if let Expr::Ident(obj_id) = obj.as_ref() {
                          let obj_name = obj_id.sym.as_ref();
                          if let Some(mut props) = self.as_obj(&obj) {
                            props.push(PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
                              key: PropName::Ident(quote_ident(&key)),
                              value: Box::new(Expr::Lit(Lit::Bool(Bool {
                                span: DUMMY_SP,
                                value: true,
                              }))),
                            }))));
                            self.idents.insert(obj_name.into(), IdentKind::Object(props));
                          } else if let Some(FnDesc { stmts, mut extends }) = self.as_function(&obj) {
                            extends.push(key.to_owned());
                            self
                              .idents
                              .insert(obj_name.into(), IdentKind::Fn(FnDesc { stmts, extends }));
                          }
                        }
                      }
                    }
                    _ => {}
                  },
                  _ => {}
                };
              }
            }
            _ => {}
          };
        }
        Stmt::Block(BlockStmt { stmts, .. }) => {
          let returned = self.walk_stmts(&stmts);
          if returned {
            return true;
          }
        }
        Stmt::If(IfStmt { test, cons, alt, .. }) => {
          let mut returned = false;
          if self.is_true(test) {
            returned = self.walk_stmts(&vec![cons.as_ref().clone()])
          } else if let Some(alt) = alt {
            returned = self.walk_stmts(&vec![alt.as_ref().clone()])
          }
          if returned {
            return true;
          }
        }
        Stmt::Return(_) => return true,
        _ => {}
      }
    }
    false
  }

  fn parse_expr(&mut self, expr: &Expr) {
    match expr {
      // exports.foo = 'bar'
      // module.exports.foo = 'bar'
      // module.exports = { foo: 'bar' }
      // module.exports = { ...require('a'), ...require('b') }
      // module.exports = require('lib')
      // foo = exports.foo || (exports.foo = {})
      Expr::Seq(SeqExpr { exprs, .. }) => {
        for expr in exprs {
          self.parse_expr(expr);
        }
      }
      Expr::Assign(assign) => {
        self.get_exports_from_assign(&assign);
      }
      // Object.defineProperty(exports, 'foo', { value: 'bar' })
      // Object.defineProperty(module.exports, 'foo', { value: 'bar' })
      // Object.defineProperty(module, 'exports', { value: { foo: 'bar' }})
      // Object.assign(exports, { foo: 'bar' })
      // Object.assign(module.exports, { foo: 'bar' }, { ...require('a') }, require('b'))
      // Object.assign(module, { exports: { foo: 'bar' } })
      // Object.assign(module, { exports: require('lib') })
      // (function() { ... })()
      // require("tslib").__exportStar(..., exports)
      // tslib.__exportStar(..., exports)
      // __exportStar(..., exports)
      Expr::Call(call) => {
        if is_object_static_mothod_call(&call, "defineProperty") && call.args.len() >= 3 {
          let arg0 = &call.args[0];
          let arg1 = &call.args[1];
          let arg2 = &call.args[2];
          let is_module = is_module_ident(arg0.expr.as_ref());
          let is_exports = self.is_exports_expr(arg0.expr.as_ref());

          let name = self.as_str(arg1.expr.as_ref());
          let mut with_value_or_getter = false;
          let mut with_value: Option<Expr> = None;
          if let Some(props) = self.as_obj(arg2.expr.as_ref()) {
            for prop in props {
              if let PropOrSpread::Prop(prop) = prop {
                let key = match prop.as_ref() {
                  Prop::KeyValue(KeyValueProp { key, value, .. }) => {
                    let key = stringify_prop_name(key);
                    if let Some(key) = &key {
                      if key.eq("value") {
                        with_value = Some(value.as_ref().clone());
                      }
                    }
                    key
                  }
                  Prop::Method(MethodProp { key, .. }) => stringify_prop_name(key),
                  _ => None,
                };
                if let Some(key) = key {
                  if key.eq("value") || key.eq("get") {
                    with_value_or_getter = true;
                    break;
                  }
                }
              }
            }
          }
          if is_exports && with_value_or_getter {
            if let Some(name) = name {
              self.exports.insert(name);
            }
          }
          if is_module {
            if let Some(expr) = with_value {
              self.reset(&expr);
            }
          }
        } else if is_object_static_mothod_call(&call, "assign") && call.args.len() >= 2 {
          let is_module = is_module_ident(call.args[0].expr.as_ref());
          let is_exports = self.is_exports_expr(call.args[0].expr.as_ref());
          for arg in &call.args[1..] {
            if let Some(props) = self.as_obj(arg.expr.as_ref()) {
              if is_module {
                let mut with_exports: Option<Expr> = None;
                for prop in props {
                  if let PropOrSpread::Prop(prop) = prop {
                    if let Prop::KeyValue(KeyValueProp { key, value, .. }) = prop.as_ref() {
                      let key = stringify_prop_name(key);
                      if let Some(key) = &key {
                        if self.is_exports_ident(key) {
                          with_exports = Some(value.as_ref().clone());
                          break;
                        }
                      }
                    };
                  }
                }
                if let Some(exports_expr) = with_exports {
                  self.reset(&exports_expr);
                }
              } else if is_exports {
                self.use_object_as_exports(props);
              }
            } else if let Some(reexport) = self.as_reexport(arg.expr.as_ref()) {
              if is_exports {
                self.reexports.insert(reexport);
              }
            }
          }
        } else if is_tslib_export_star_call(&call) && call.args.len() >= 2 {
          let is_exports = self.is_exports_expr(call.args[1].expr.as_ref());
          if is_exports {
            if let Some(props) = self.as_obj(call.args[0].expr.as_ref()) {
              self.use_object_as_exports(props);
            } else if let Some(reexport) = self.as_reexport(call.args[0].expr.as_ref()) {
              self.reexports.insert(reexport);
            }
          }
        } else if is_export_call(&call) && call.args.len() > 0 {
          if let Some(props) = self.as_obj(call.args[0].expr.as_ref()) {
            self.use_object_as_exports(props);
          } else if let Some(reexport) = self.as_reexport(call.args[0].expr.as_ref()) {
            self.reexports.insert(reexport);
          }
        } else if let Some(body) = self.is_umd_iife_call(&call) {
          self.walk_body(body, false);
        } else if let Some(body) = is_iife_call(&call) {
          for arg in &call.args {
            if arg.spread.is_none() {
              // (function() { ... })(exports.foo || (exports.foo = {}))
              if let Some(bare_export_name) = self.get_bare_export_names(arg.expr.as_ref()) {
                self.exports.insert(bare_export_name);
              }
            }
          }
          self.walk_body(body, false);
        }
      }
      // ~function(){ ... }()
      // !(function(e, t) { ... })(this, (function (e) { ... }));
      Expr::Unary(UnaryExpr { op, arg, .. }) => {
        if let UnaryOp::Minus | UnaryOp::Plus | UnaryOp::Bang | UnaryOp::Tilde | UnaryOp::Void = op {
          if let Expr::Call(call) = arg.as_ref() {
            if let Some(body) = self.is_umd_iife_call(&call) {
              self.walk_body(body, false);
            } else if let Some(body) = is_iife_call(&call) {
              // (function() { ... })(exports.foo || (exports.foo = {}))
              for arg in &call.args {
                if arg.spread.is_none() {
                  if let Some(bare_export_name) = self.get_bare_export_names(arg.expr.as_ref()) {
                    self.exports.insert(bare_export_name);
                  }
                }
              }
              self.walk_body(body, false);
            }
          }
        }
      }
      // (function(){ ... }())
      Expr::Paren(ParenExpr { expr, .. }) => {
        self.walk(
          vec![Stmt::Expr(ExprStmt {
            span: DUMMY_SP,
            expr: expr.clone(),
          })],
          false,
        );
      }
      // 0 && (module.exports = { foo })
      Expr::Bin(BinExpr { op, right, .. }) => {
        if matches!(op, BinaryOp::LogicalAnd) {
          if let Expr::Assign(assign) = right.as_ref() {
            self.get_exports_from_assign(assign);
          } else if let Expr::Paren(paren) = right.as_ref() {
            if let Expr::Assign(assign) = paren.expr.as_ref() {
              self.get_exports_from_assign(assign);
            }
          }
        }
      }
      _ => {}
    }
  }

  fn walk(&mut self, stmts: Vec<Stmt>, as_fn: bool) {
    self.walk_stmts(&stmts);

    // check exports (as function)
    if as_fn {
      for stmt in &stmts {
        if self.fn_returned {
          break;
        }
        match stmt {
          Stmt::Block(BlockStmt { stmts, .. }) => {
            self.walk_body(stmts.clone(), true);
          }
          Stmt::If(IfStmt { test, cons, alt, .. }) => {
            if self.is_true(test) {
              self.walk_body(vec![cons.as_ref().clone()], true);
            } else if let Some(alt) = alt {
              self.walk_body(vec![alt.as_ref().clone()], true);
            }
          }
          Stmt::Return(ReturnStmt { arg, .. }) => {
            self.fn_returned = true;
            if let Some(arg) = arg {
              self.reset(arg);
            }
          }
          _ => {}
        }
      }
      return;
    }

    // check exports
    for stmt in &stmts {
      match stmt {
        // var foo = exports.foo || (exports.foo = {})
        Stmt::Decl(Decl::Var(var)) => {
          for decl in var.as_ref().decls.iter() {
            self.try_to_mark_exports_alias(decl);
            if let Some(init_expr) = &decl.init {
              if let Some(bare_export_name) = self.get_bare_export_names(init_expr) {
                self.exports.insert(bare_export_name);
              }
            }
          }
        }
        Stmt::Expr(ExprStmt { expr, .. }) => self.parse_expr(expr),
        Stmt::Block(BlockStmt { stmts, .. }) => {
          self.walk_body(stmts.clone(), false);
        }
        Stmt::If(IfStmt { test, cons, alt, .. }) => {
          if self.is_true(test) {
            self.walk_body(vec![cons.as_ref().clone()], false);
          } else if let Some(alt) = alt {
            self.walk_body(vec![alt.as_ref().clone()], false);
          }
        }
        Stmt::Return(ReturnStmt { arg, .. }) => {
          if let Some(arg) = arg {
            match &**arg {
              Expr::Call(call) => match with_expr_callee(call) {
                Some(Expr::Fn(FnExpr { function, .. })) => {
                  if let Function {
                    body: Some(BlockStmt { stmts, .. }),
                    ..
                  } = function.as_ref()
                  {
                    let mut check_function = |function: &Box<Function>| {
                      if let Function {
                        body: Some(BlockStmt { stmts, .. }),
                        ..
                      } = function.as_ref()
                      {
                        if let Some(Stmt::If(IfStmt { cons, .. })) = stmts.get(0) {
                          if let Stmt::Return(ReturnStmt { arg: Some(arg), .. }) = &**cons {
                            if let Expr::Member(MemberExpr {
                              prop: MemberProp::Ident(prop),
                              ..
                            }) = &**arg
                            {
                              if prop.sym.as_ref().eq("exports") {
                                if call.args.len() != 1 {
                                  return;
                                }
                                if let Some(ExprOrSpread { expr, .. }) = call.args.get(0) {
                                  if let Expr::Array(ArrayLit { elems, .. }) = &**expr {
                                    for elem in elems {
                                      if let Some(ExprOrSpread { expr, .. }) = elem {
                                        if let Expr::Fn(FnExpr { function, .. }) = &**expr {
                                          if let Function {
                                            body: Some(BlockStmt { stmts, .. }),
                                            params,
                                            ..
                                          } = function.as_ref()
                                          {
                                            if let Some(Param {
                                              pat:
                                                Pat::Ident(BindingIdent {
                                                  id:
                                                    Ident {
                                                      sym: webpack_exports_sym,
                                                      ..
                                                    },
                                                  ..
                                                }),
                                              ..
                                            }) = params.get(1)
                                            {
                                              if let Some(Param {
                                                pat:
                                                  Pat::Ident(BindingIdent {
                                                    id:
                                                      Ident {
                                                        sym: webpack_require_sym,
                                                        ..
                                                      },
                                                    ..
                                                  }),
                                                ..
                                              }) = params.get(2)
                                              {
                                                for stmt in stmts {
                                                  if let Stmt::Expr(ExprStmt { expr, .. }) = stmt {
                                                    self.get_webpack4_exports(
                                                      expr,
                                                      webpack_exports_sym.as_ref(),
                                                      Some(webpack_require_sym.as_ref()),
                                                    )
                                                  }
                                                }
                                              } else {
                                                for stmt in stmts {
                                                  if let Stmt::Expr(ExprStmt { expr, .. }) = stmt {
                                                    self.get_webpack4_exports(expr, webpack_exports_sym.as_ref(), None)
                                                  }
                                                }
                                              }
                                            }
                                          }
                                        }
                                      }
                                    }
                                  }
                                }
                              }
                            }
                          }
                        }
                      }
                    };
                    if let Some(Stmt::Decl(Decl::Fn(FnDecl { function, .. }))) = stmts.get(0) {
                      check_function(function);
                    } else if let Some(Stmt::Decl(Decl::Fn(FnDecl { function, .. }))) = stmts.get(1) {
                      check_function(function);
                    }
                  }
                }
                Some(Expr::Paren(ParenExpr { expr, .. })) => {
                  if let Expr::Arrow(ArrowExpr { body, .. }) = &**expr {
                    if let BlockStmtOrExpr::BlockStmt(BlockStmt { stmts, .. }) = &**body {
                      let first_stmt_index = match stmts.get(0) {
                        Some(Stmt::Expr(ExprStmt { expr, .. })) => match &**expr {
                          Expr::Lit(Lit::Str(Str { value, .. })) => {
                            if value.to_string().eq("use strict") {
                              1
                            } else {
                              0
                            }
                          }
                          _ => 0,
                        },
                        _ => 0,
                      };

                      if let Some(Stmt::Decl(Decl::Var(var_decl))) = stmts.get(first_stmt_index) {
                        let VarDecl { decls, .. } = &**var_decl;
                        match decls.get(0) {
                          Some(VarDeclarator {
                            name:
                              Pat::Ident(BindingIdent {
                                id:
                                  Ident {
                                    sym: webpack_require_sym,
                                    ..
                                  },
                                ..
                              }),
                            init,
                            ..
                          }) => {
                            if let Some(init) = init {
                              if let Expr::Object(ObjectLit { props, .. }) = &**init {
                                let webpack_require_props = self.get_webpack_require_props_from_props(props);

                                if webpack_require_props == 2 {
                                  self.get_webpack_exports(stmts, &webpack_require_sym, &(first_stmt_index + 1));
                                }
                              }
                            }
                          }
                          _ => {}
                        }
                      }

                      if let Some(Stmt::Decl(Decl::Var(var_decl))) = stmts.get(first_stmt_index) {
                        let VarDecl { decls, .. } = &**var_decl;
                        match decls.get(0) {
                          Some(VarDeclarator {
                            name:
                              Pat::Ident(BindingIdent {
                                id:
                                  Ident {
                                    sym: webpack_require_sym,
                                    ..
                                  },
                                ..
                              }),
                            init,
                            ..
                          }) => {
                            if let Some(init) = init {
                              if let Expr::Object(ObjectLit { props, .. }) = &**init {
                                let webpack_require_props = self.get_webpack_require_props_from_props(props);

                                if webpack_require_props == 2 {
                                  self.get_webpack_exports(stmts, &webpack_require_sym, &(first_stmt_index + 1));
                                }
                              }
                            }
                          }
                          _ => {}
                        }
                      }

                      if let Some(Stmt::Decl(Decl::Fn(FnDecl {
                        ident:
                          Ident {
                            sym: webpack_require_sym,
                            ..
                          },
                        ..
                      }))) = stmts.get(first_stmt_index + 1)
                      {
                        let webpack_require_props =
                          self.get_webpack_require_props_from_stmts(stmts, webpack_require_sym);
                        if webpack_require_props == 2 {
                          if let Some(Stmt::Return(ReturnStmt { arg: Some(arg), .. })) = stmts.get(stmts.len() - 1) {
                            if let Expr::Seq(SeqExpr { exprs, .. }) = &**arg {
                              if let Some(expr) = exprs.get(0) {
                                if let Expr::Call(call) = &**expr {
                                  if let Some(stmts) = is_iife_call(call) {
                                    self.get_webpack_exports(&stmts, &webpack_require_sym, &0);
                                  }
                                }
                              }
                            }
                          }
                        }
                      }

                      if let Some(Stmt::Return(ReturnStmt { arg, .. })) = stmts.get(stmts.len() - 1) {
                        if let Some(arg) = arg {
                          match &**arg {
                            Expr::Seq(SeqExpr { exprs, .. }) => {
                              if let Some(module_exports_expr) = exprs.get(exprs.len() - 1) {
                                if let Some(module_iife_expr) = exprs.get(0) {
                                  if let Expr::Call(module_iife_call_expr) = &**module_iife_expr {
                                    if let Some(stmts) = is_iife_call(module_iife_call_expr) {
                                      if let Expr::Ident(Ident {
                                        sym: module_exports_sym,
                                        ..
                                      }) = &**module_exports_expr
                                      {
                                        if let Some(Stmt::Decl(Decl::Var(var_decl))) = stmts.get(0) {
                                          let VarDecl { decls, .. } = &**var_decl;
                                          if let Some(VarDeclarator { name, init, .. }) = decls.get(0) {
                                            if let Some(init_expr) = init {
                                              if let Expr::Ident(Ident { sym, .. }) = &**init_expr {
                                                if module_exports_sym.as_ref().eq(sym.as_ref()) {
                                                  if let Pat::Ident(BindingIdent {
                                                    id: Ident { sym, .. }, ..
                                                  }) = name
                                                  {
                                                    self.exports_alias.insert(sym.as_ref().to_owned());
                                                    self.walk_body(stmts, false);
                                                    return;
                                                  }
                                                }
                                              }
                                            }
                                          }
                                        }

                                        if let Some(Stmt::Decl(Decl::Var(var_decl))) = stmts.get(1) {
                                          let VarDecl { decls, .. } = &**var_decl;
                                          if let Some(VarDeclarator { name, init, .. }) = decls.get(0) {
                                            if let Some(init_expr) = init {
                                              if let Expr::Ident(Ident { sym, .. }) = &**init_expr {
                                                if module_exports_sym.as_ref().eq(sym.as_ref()) {
                                                  if let Pat::Ident(BindingIdent {
                                                    id: Ident { sym, .. }, ..
                                                  }) = name
                                                  {
                                                    self.exports_alias.insert(sym.as_ref().to_owned());
                                                    self.walk_body(stmts, false);
                                                    return;
                                                  }
                                                }
                                              }
                                            }
                                          }
                                        }
                                      }
                                    }
                                  }
                                }
                              }
                            }
                            _ => {}
                          }
                        }
                      }
                    }
                  }
                }
                _ => {}
              },
              _ => {}
            }
          }
        }
        _ => {}
      }
    }
  }

  fn walk_body(&mut self, body: Vec<Stmt>, as_fn: bool) {
    let mut lexer = CJSLexer {
      node_env: self.node_env.to_owned(),
      call_mode: false,
      fn_returned: false,
      idents: self.idents.clone(),
      exports_alias: self.exports_alias.clone(),
      exports: self.exports.clone(),
      reexports: self.reexports.clone(),
    };
    lexer.walk(body, as_fn);
    self.fn_returned = lexer.fn_returned;
    self.exports = lexer.exports;
    self.reexports = lexer.reexports;
  }
}

impl Fold for CJSLexer {
  noop_fold_type!();

  fn fold_module_items(&mut self, items: Vec<ModuleItem>) -> Vec<ModuleItem> {
    let stmts = items
      .iter()
      .filter(|&item| match item {
        ModuleItem::Stmt(_) => true,
        _ => false,
      })
      .map(|item| match item {
        ModuleItem::Stmt(stmt) => stmt.clone(),
        _ => Stmt::Empty(EmptyStmt { span: DUMMY_SP }),
      })
      .filter(|item| !item.is_empty())
      .collect::<Vec<Stmt>>();
    self.walk(stmts, false);
    items
  }
}

fn is_module_ident(expr: &Expr) -> bool {
  match expr {
    Expr::Ident(id) => {
      let id = id.sym.as_ref();
      id.eq("module")
    }
    _ => false,
  }
}

fn is_member(expr: &Expr, obj_name: &str, prop_name: &str) -> bool {
  if let Some(member_prop_name) = get_member_prop_name(expr, obj_name) {
    return member_prop_name.eq(prop_name);
  }
  false
}

fn is_member_member(expr: &Expr, obj_name: &str, middle_obj_name: &str, prop_name: &str) -> bool {
  if let Expr::Member(MemberExpr { obj, prop, .. }) = expr {
    if is_member(obj, obj_name, middle_obj_name) {
      if let Some(name) = get_prop_name(prop) {
        return name.eq(prop_name);
      }
    }
  }
  false
}

fn with_expr_callee(call: &CallExpr) -> Option<&Expr> {
  match &call.callee {
    Callee::Expr(callee) => Some(callee.as_ref()),
    _ => None,
  }
}

// require('lib')
fn is_require_call(call: &CallExpr) -> Option<String> {
  if let Some(Expr::Ident(id)) = with_expr_callee(call) {
    if id.sym.as_ref().eq("require") && call.args.len() > 0 {
      return match call.args[0].expr.as_ref() {
        Expr::Lit(Lit::Str(Str { value, .. })) => Some(value.as_ref().to_owned()),
        _ => None,
      };
    }
  };
  None
}

// match:
// Object.defineProperty()
// Object.assgin()
fn is_object_static_mothod_call(call: &CallExpr, method: &str) -> bool {
  if let Some(callee) = with_expr_callee(call) {
    return is_member(callee, "Object", method);
  }
  false
}

fn is_umd_params(params: &Vec<Pat>) -> bool {
  if params.len() == 2 {
    if let Pat::Ident(bid) = &params.get(0).unwrap() {
      if bid.id.sym.eq("global") {
        if let Pat::Ident(bid) = &params.get(1).unwrap() {
          if bid.id.sym.eq("factory") {
            return true;
          }
        }
      }
    }
  }
  false
}

fn is_string_literal(expr: &Expr, value: &str) -> bool {
  match &*expr {
    Expr::Lit(Lit::Str(Str {
      value: literal_value, ..
    })) => literal_value.eq(value),
    _ => false,
  }
}

fn is_typeof(expr: &Expr, identifier: &str) -> bool {
  match &*expr {
    Expr::Unary(UnaryExpr { arg, op, .. }) => match op {
      UnaryOp::TypeOf => match &**arg {
        Expr::Ident(ident) => ident.sym.eq(identifier),
        _ => false,
      },
      _ => false,
    },
    _ => false,
  }
}

fn is_umd_exports_check(expr: &Expr) -> bool {
  match &*expr {
    Expr::Bin(BinExpr { left, right, op, .. }) => match op {
      BinaryOp::EqEq | BinaryOp::EqEqEq => {
        (is_typeof(&left, "exports") && is_string_literal(&right, "object"))
          || (is_typeof(&right, "exports") && is_string_literal(&left, "object"))
      }
      _ => false,
    },
    _ => false,
  }
}

fn is_umd_module_check(expr: &Expr) -> bool {
  match &*expr {
    Expr::Bin(BinExpr { left, right, op, .. }) => match op {
      BinaryOp::EqEq | BinaryOp::EqEqEq => {
        (is_typeof(&left, "module") && is_string_literal(&right, "object"))
          || (is_typeof(&right, "module") && is_string_literal(&left, "object"))
      }
      BinaryOp::NotEq | BinaryOp::NotEqEq => {
        (is_typeof(&left, "module") && is_string_literal(&right, "undefined"))
          || (is_typeof(&right, "module") && is_string_literal(&left, "undefined"))
      }
      _ => false,
    },
    _ => false,
  }
}

fn is_umd_define_check(expr: &Expr) -> bool {
  match &*expr {
    Expr::Bin(BinExpr { left, right, op, .. }) => match op {
      BinaryOp::EqEq | BinaryOp::EqEqEq => {
        (is_typeof(&left, "define") && is_string_literal(&right, "function"))
          || (is_typeof(&right, "define") && is_string_literal(&left, "function"))
      }
      _ => false,
    },
    _ => false,
  }
}

// if ('object' == typeof exports && 'object' == typeof module)
// module.exports = t(require('react'));
// else if ('function' == typeof define && define.amd) define(['react'], t);
// else {
// var r = 'object' == typeof exports ? t(require('react')) : t(e.react);
// for (var n in r) ('object' == typeof exports ? exports : e)[n] = r[n];
// }
fn is_umd_checks(stmts: &Vec<Stmt>) -> bool {
  match stmts.get(0) {
    Some(stmt) => match stmt {
      // TODO: handle ternary version
      // !(function (e, t) {
      //   "object" == typeof exports && "undefined" != typeof module
      //     ? t(exports)
      //     : "function" == typeof define && define.amd
      //     ? define(["exports"], t)
      //     : t(
      //         ((e =
      //           "undefined" != typeof globalThis
      //             ? globalThis
      //             : e || self).rudderanalytics = {})
      //       );
      // })(this, function (e) {
      Stmt::Expr(ExprStmt { expr, .. }) => match &**expr {
        Expr::Cond(CondExpr { test, alt, .. }) => match &**test {
          Expr::Bin(BinExpr { left, right, op, .. }) => {
            if matches!(op, BinaryOp::LogicalAnd) {
              if (is_umd_exports_check(&left) && is_umd_module_check(&right))
                || (is_umd_exports_check(&right) && is_umd_module_check(&left))
              {
                match &**alt {
                  Expr::Cond(CondExpr { test, .. }) => match &**test {
                    Expr::Bin(BinExpr { left, op, .. }) => {
                      if matches!(op, BinaryOp::LogicalAnd) && is_umd_define_check(&left) {
                        return true;
                      }
                      return false;
                    }
                    _ => false,
                  },
                  _ => false,
                };
              }
              return false;
            }
            return false;
          }
          _ => false,
        },
        _ => false,
      },
      Stmt::If(IfStmt { test, alt, .. }) => match &**test {
        Expr::Bin(BinExpr { left, right, op, .. }) => {
          if matches!(op, BinaryOp::LogicalAnd) {
            if (is_umd_exports_check(&left) && is_umd_module_check(&right))
              || (is_umd_exports_check(&right) && is_umd_module_check(&left))
            {
              match &*alt {
                Some(alt_stmt) => match &**alt_stmt {
                  Stmt::If(IfStmt { test, .. }) => match &**test {
                    Expr::Bin(BinExpr { left, op, .. }) => {
                      if matches!(op, BinaryOp::LogicalAnd) && is_umd_define_check(&left) {
                        return true;
                      }
                      return false;
                    }
                    _ => false,
                  },
                  _ => false,
                },
                _ => false,
              };
            }
            return false;
          }
          return false;
        }
        _ => false,
      },
      _ => false,
    },
    None => false,
  }
}

fn is_iife_call(call: &CallExpr) -> Option<Vec<Stmt>> {
  let expr = if let Some(callee) = with_expr_callee(call) {
    match callee {
      Expr::Paren(ParenExpr { expr, .. }) => expr.as_ref(),
      _ => callee,
    }
  } else {
    return None;
  };
  match expr {
    Expr::Fn(func) => {
      if let Some(BlockStmt { stmts, .. }) = &func.function.body {
        return Some(stmts.clone());
      }
    }
    Expr::Arrow(arrow) => return Some(get_arrow_body_as_stmts(arrow)),
    _ => {}
  }
  None
}

fn is_export_call(call: &CallExpr) -> bool {
  if let Some(callee) = with_expr_callee(call) {
    match callee {
      Expr::Ident(id) => {
        return id.sym.as_ref().eq("__export");
      }
      _ => {}
    }
  }
  false
}

// match:
// require("tslib").__exportStar(..., exports)
// (0, require("tslib").__exportStar)(..., exports)
// const tslib = require("tslib"); (0, tslib.__exportStar)(..., exports)
// const {__exportStar} = require("tslib"); (0, __exportStar)(..., exports)
// const __exportStar = () => {}; __exportStar(..., exports)
fn is_tslib_export_star_call(call: &CallExpr) -> bool {
  if let Some(callee) = with_expr_callee(call) {
    match callee {
      Expr::Member(MemberExpr { prop, .. }) => {
        if let MemberProp::Ident(prop) = prop {
          return prop.sym.as_ref().eq("__exportStar");
        }
      }
      Expr::Ident(id) => {
        return id.sym.as_ref().eq("__exportStar");
      }
      Expr::Paren(ParenExpr { expr, .. }) => match expr.as_ref() {
        Expr::Member(MemberExpr { prop, .. }) => {
          if let MemberProp::Ident(prop) = prop {
            return prop.sym.as_ref().eq("__exportStar");
          }
        }
        Expr::Ident(id) => {
          return id.sym.as_ref().eq("__exportStar");
        }
        Expr::Seq(SeqExpr { exprs, .. }) => {
          if let Some(last) = exprs.last() {
            match last.as_ref() {
              Expr::Member(MemberExpr { prop, .. }) => {
                if let MemberProp::Ident(prop) = prop {
                  return prop.sym.as_ref().eq("__exportStar");
                }
              }
              Expr::Ident(id) => {
                return id.sym.as_ref().eq("__exportStar");
              }
              _ => {}
            }
          }
        }
        _ => {}
      },
      _ => {}
    }
  }
  false
}

fn get_member_expr_from_assign_target(v: &AssignTarget) -> Option<&MemberExpr> {
  match v {
    AssignTarget::Simple(s) => match s {
      SimpleAssignTarget::Member(member) => Some(member),
      _ => None,
    },
    _ => None,
  }
}

fn get_arrow_body_as_stmts(arrow: &ArrowExpr) -> Vec<Stmt> {
  match &*arrow.body {
    BlockStmtOrExpr::BlockStmt(BlockStmt { stmts, .. }) => stmts.clone(),
    BlockStmtOrExpr::Expr(expr) => vec![Stmt::Return(ReturnStmt {
      span: DUMMY_SP,
      arg: Some(expr.clone()),
    })],
  }
}

fn get_member_prop_name(expr: &Expr, obj_name: &str) -> Option<String> {
  if let Expr::Member(MemberExpr { obj, prop, .. }) = expr {
    if let Expr::Ident(obj) = obj.as_ref() {
      if obj.sym.as_ref().eq(obj_name) {
        return get_prop_name(prop);
      }
    }
  }
  None
}

fn get_class_static_names(class: &Class) -> Vec<String> {
  class
    .body
    .iter()
    .filter(|&member| match member {
      ClassMember::ClassProp(prop) => prop.is_static,
      ClassMember::Method(method) => method.is_static,
      _ => false,
    })
    .map(|member| {
      match member {
        ClassMember::ClassProp(prop) => {
          if let PropName::Ident(id) = &prop.key {
            return id.sym.as_ref().into();
          }
        }
        ClassMember::Method(method) => {
          if let PropName::Ident(id) = &method.key {
            return id.sym.as_ref().into();
          }
        }
        _ => {}
      };
      "".to_owned()
    })
    .collect()
}

fn get_prop_name(prop: &MemberProp) -> Option<String> {
  match prop {
    MemberProp::Ident(prop) => Some(prop.sym.as_ref().into()),
    MemberProp::Computed(ComputedPropName { expr, .. }) => match expr.as_ref() {
      Expr::Ident(prop) => Some(prop.sym.as_ref().into()),
      Expr::Lit(Lit::Str(Str { value, .. })) => Some(value.as_ref().into()),
      _ => None,
    },
    _ => None,
  }
}

fn stringify_prop_name(name: &PropName) -> Option<String> {
  match name {
    PropName::Ident(id) => Some(id.sym.as_ref().into()),
    PropName::Str(Str { value, .. }) => Some(value.as_ref().into()),
    _ => None,
  }
}

fn quote_ident(value: &str) -> Ident {
  Ident {
    span: DUMMY_SP,
    sym: value.into(),
    optional: false,
  }
}

fn quote_str(value: &str) -> Str {
  Str {
    span: DUMMY_SP,
    value: value.into(),
    raw: None,
  }
}
