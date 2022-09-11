ThisBuild / version := "1.0.0"
ThisBuild / scalaVersion := "3.2.0"
ThisBuild / organization := "click.seichi"

mainClass := Some("click.seichi")

assemblyJarName := {
  s"${name.value}-${version.value}.jar"
}

libraryDependencies ++= Seq(
  "org.typelevel" %% "cats-effect" % "3.3.14",
  "org.http4s" %% "http4s-dsl" % "0.23.15"
)

lazy val root =
  (project in file(".")).settings(
    name := "SeichiTimedStatsConifers",
    // scalafixがsemanticdbを要求するので設定
    semanticdbEnabled := true,
    // これがないと各.classファイルに対してsemanticdbファイルが生成されてしまう
    semanticdbIncludeInJar := true
  )
