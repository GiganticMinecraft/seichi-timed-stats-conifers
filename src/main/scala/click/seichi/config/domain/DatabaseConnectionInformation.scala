package click.seichi.config.domain

case class DatabaseConnectionInformation(
  host: String,
  user: String,
  password: String,
  port: Int
) {
  require(host != null, "ホスト名が不正です。")
  require(user != null, "ユーザー名が不正です。")
  require(password != null, "パスワードが不正です。")
  // ポートの範囲についての参考ページ: https://www.ibm.com/docs/ja/i/7.3?topic=ssw_ibm_i_73/cl/addtcpport.htm
  require(1 <= port && port <= 65535, "ポートは1以上65535以下で指定してください。")
}
