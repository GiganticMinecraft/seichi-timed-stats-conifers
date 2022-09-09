/*
  sbt-assemblyプラグインは2022/09/08現在Scala3に対応しているということが記載されていないが、
  実際に動かしてみると問題がなさそうなのでとりあえずsbt-assemblyプラグインを利用する。
  問題があれば代替方法を利用してください。

  see: https://github.com/sbt/sbt-assembly
 */
addSbtPlugin("com.eed3si9n" % "sbt-assembly" % "1.2.0")

/*
  Scalafixプラグインは2022/09/09現在Scala3への対応が試験的なものとなっている。
  そのため、問題が発生したらScalafixにPRをなげるなどの「強い意志」をもって利用してください。

  Scalafix repository: https://github.com/scalacenter/scalafix
 */
addSbtPlugin("ch.epfl.scala" % "sbt-scalafix" % "0.10.1")
