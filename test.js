const fs = require('fs')
const { parse } = require('./pkg/esm_cjs_lexer')

exports.foo = true
module.exports.bar = true

if (process.env.NODE_ENV === 'development') {
  exports.dev = true
} else {
  exports.prod = true
}

const code = fs.readFileSync("./test.js", "utf-8")

const ret = parse("./test.js", code, { nodeEnv: "development" })
if (ret.exports.join(',') !== 'foo,bar,dev') {
  throw new Error('unexpected exports of index.js: ' + ret.exports.join(','))
}

const ret2 = parse("./test.js", code, { nodeEnv: "production" })
if (ret2.exports.join(',') !== 'foo,bar,prod') {
  throw new Error('unexpected exports of index.js: ' + ret2.exports.join(','))
}

console.log("done")
