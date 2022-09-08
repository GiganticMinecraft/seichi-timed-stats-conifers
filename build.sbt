ThisBuild / version := "1.0.0"
ThisBuild / scalaVersion := "3.2.0"
ThisBuild / organization := "click.seichi"

mainClass := Some("click.seichi")

assemblyJarName := {
  s"${name.value}-${version.value}.jar"
}

lazy val root = (project in file("."))
  .settings(
    name := "SeichiTimedStatsConifers"
  )
