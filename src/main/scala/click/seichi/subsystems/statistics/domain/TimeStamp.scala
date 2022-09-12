package click.seichi.subsystems.statistics.domain

import java.time.LocalDateTime
import java.time.format.DateTimeFormatter

case class TimeStamp(value: LocalDateTime)

object TimeStamp {

  /**
   * @return yyyy/MM/dd HH:mm:ssの形式でフォーマットされた文字列から[[TimeStamp]]を生成して返す
   */
  def fromString(stringDateTime: String): TimeStamp = {
    val dateTimeFormatter = DateTimeFormatter.ofPattern("yyyy/MM/dd HH:mm:ss")
    val dateTime = LocalDateTime.parse(stringDateTime, dateTimeFormatter)

    TimeStamp(dateTime)
  }

  /**
   * @return 現在の時刻から[[TimeStamp]]を生成して返す作用
   */
  def fromNow: TimeStamp = {
    val dateTime = LocalDateTime.now()

    TimeStamp(dateTime)
  }

}
