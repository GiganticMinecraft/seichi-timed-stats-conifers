/*
  sbt-assemblyプラグインは2022/09/08現在Scala3に対応しているということが記載されていないが、
  実際に動かしてみると問題がなさそうなのでとりあえずsbt-assemblyプラグインを利用する。
  問題があれば代替方法を利用してください。

  see: https://github.com/sbt/sbt-assembly
 */
addSbtPlugin("com.eed3si9n" % "sbt-assembly" % "1.2.0")