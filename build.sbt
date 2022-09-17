ThisBuild / version := "1.0.0"
ThisBuild / scalaVersion := "3.2.0"
ThisBuild / organization := "click.seichi"

mainClass := Some("click.seichi")

assemblyJarName := {
  s"${name.value}-${version.value}.jar"
}

libraryDependencies ++= Seq(
  "org.typelevel" %% "cats-effect" % "3.3.14",
  "org.http4s" %% "http4s-dsl" % "0.23.15",
  "org.flywaydb" % "flyway-core" % "9.3.0",
  "org.scalikejdbc" %% "scalikejdbc" % "4.0.0",
  "com.typesafe" % "config" % "1.4.2"
)

lazy val root =
  (project in file(".")).settings(name := "SeichiTimedStatsConifers", semanticdbEnabled := true)
