use std::time::{Duration, Instant};

use esp_idf_hal::{gpio::*, peripheral::Peripheral};

// 双击(连续两次的点击)间隔
const DOUBLE_PRESSED_DURATION: Duration = Duration::from_millis(200);

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Ec11Direction {
    /* 顺时针旋转 */
    Cw,
    /* 逆时针旋转 */
    Ccw,
}

#[derive(Debug, Clone, Copy)]
pub enum Ec11RotateStatus {
    /* 顺时针旋转 */
    CwStart,
    CwEnd,
    /* 逆时针旋转 */
    CcwStart,
    CcwEnd,
}

#[derive(Debug, Clone, Copy)]
pub enum Ec11PressStatus {
    /* 按下 */
    Pressed,
    /* 释放 */
    Released,
    /* 再次按下 */
    TwicePressed,
    /* 再次释放 */
    TwiceReleased,
}

pub type Ec11Position = i32;
pub type Ec11PositionDelta = i8;

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum Ec11Event {
    Clicked,
    DoubleClicked,
    // LongClicked,
    ClickedRotate(Ec11Direction, Ec11Position, Ec11PositionDelta),
    Rotate(Ec11Direction, Ec11Position, Ec11PositionDelta),
    Empty,
}

pub fn ec11_service(
    ec11_key: impl Peripheral<P = impl IOPin> + 'static,
    ec11_a: impl Peripheral<P = impl IOPin> + 'static,
    ec11_b: impl Peripheral<P = impl IOPin> + 'static,
) {
    /* 配置 gpio */

    let mut ec11_key = PinDriver::input(ec11_key).unwrap();
    let mut ec11_a = PinDriver::input(ec11_a).unwrap();
    let mut ec11_b = PinDriver::input(ec11_b).unwrap();

    ec11_key.set_pull(Pull::Up).unwrap();
    ec11_a.set_pull(Pull::Up).unwrap();
    ec11_b.set_pull(Pull::Up).unwrap();

    let (event_tx, event_rx) = std::sync::mpsc::channel::<Ec11Event>();

    /* 扫描信号 */

    std::thread::spawn(move || {
        let start = Instant::now();
        // 一次旋转的位置
        let mut rotate_position = 0i32;

        // 前一次 a 的状态
        let mut ec11_a_previous = ec11_a.get_level();
        // 前一次 key 的状态
        let mut ec11_key_previous = ec11_key.get_level();
        // 第一次释放的时刻, 用于确定 双击事件
        let mut first_released_moment = start.elapsed();

        let mut has_pressed = false;
        let mut has_rotate_in_pressed = false;
        let mut has_twice_pressed = false;

        loop {
            // 计算出的当前旋转的状态
            let mut rotate_status = None;
            // 计算出的当前按下的状态
            let mut press_status = None;

            /* 处理A B 两个信号 */
            if ec11_a.get_level() != ec11_a_previous {
                std::thread::sleep(Duration::from_millis(1));

                /*
                    把 a(ec11_a) 当做时钟信号, b(ec11_b) 当做数据信号,
                    正转:
                        当 a 下降为0时, 如果 b为1, 则为正转开始
                        当 a 上升为1时, 如果 b为0, 这为正转结束
                    正转:
                        当 a 下降为0时, 如果 b为0, 则为反转开始
                        当 a 上升为1时, 如果 b为1, 这为反转结束
                */

                // 连续的时间内, 很可能存在ns/us级的抖动, 休眠一定时间后 状态稳定下来, 再次读取, 以此避免抖动
                let a_level = ec11_a.get_level();

                if a_level != ec11_a_previous {
                    // 更新状态
                    ec11_a_previous = a_level;

                    // ec11_a 下降沿
                    if a_level == Level::Low {
                        if ec11_b.get_level() == Level::High {
                            rotate_status = Some(Ec11RotateStatus::CwStart);
                        } else {
                            rotate_status = Some(Ec11RotateStatus::CcwStart);
                        }
                    }
                    // ec11_a 上升沿
                    else {
                        if ec11_b.get_level() == Level::Low {
                            rotate_status = Some(Ec11RotateStatus::CwEnd);
                            rotate_position = rotate_position.wrapping_add(1);
                        } else {
                            rotate_status = Some(Ec11RotateStatus::CcwEnd);
                            rotate_position = rotate_position.wrapping_sub(1);
                        }
                    }
                }
            }

            if ec11_key.get_level() != ec11_key_previous {
                std::thread::sleep(Duration::from_millis(1));
                // 连续的时间内, 很可能存在ns/us级的抖动, 休眠一定时间后 状态稳定下来, 再次读取, 以此避免抖动
                let key_level = ec11_key.get_level();
                if key_level != ec11_key_previous {
                    // 更改状态
                    ec11_key_previous = ec11_key.get_level();

                    // 按下
                    if key_level == Level::Low {
                        press_status = Some(Ec11PressStatus::Pressed);
                        has_pressed = true;

                        // 双击: 连续按键的间隔小于阈值
                        if start.elapsed() - first_released_moment <= DOUBLE_PRESSED_DURATION {
                            press_status = Some(Ec11PressStatus::TwicePressed);
                            has_twice_pressed = true;
                        }
                    }
                    // 释放
                    else {
                        press_status = Some(Ec11PressStatus::Released);
                        has_pressed = false;

                        // 如果已经按下了两次
                        if has_twice_pressed {
                            has_twice_pressed = false;
                            press_status = Some(Ec11PressStatus::TwiceReleased);
                        }
                        // 第一次释放
                        else {
                            first_released_moment = start.elapsed();
                        }
                    }
                }
            }

            // 由于按下只是某一个瞬间的事情, 保持按下的时候 press_status 是为 None 的, 因为 key 前后状态没有发生改变
            if press_status.is_none() && has_pressed {
                press_status = Some(Ec11PressStatus::Pressed);
            }

            if has_pressed && rotate_status.is_some() {
                has_rotate_in_pressed = true;
            }

            let event = match (rotate_status, press_status) {
                (None, None) => Ec11Event::Empty,
                (None, Some(Ec11PressStatus::Released)) => {
                    if has_rotate_in_pressed {
                        Ec11Event::Empty
                    } else {
                        Ec11Event::Clicked
                    }
                }
                (None, Some(Ec11PressStatus::TwiceReleased)) => Ec11Event::DoubleClicked,
                (Some(Ec11RotateStatus::CwEnd), None) => {
                    Ec11Event::Rotate(Ec11Direction::Cw, rotate_position, 1)
                }
                (Some(Ec11RotateStatus::CcwEnd), None) => {
                    Ec11Event::Rotate(Ec11Direction::Ccw, rotate_position, -1)
                }
                (Some(Ec11RotateStatus::CwEnd), Some(Ec11PressStatus::Pressed)) => {
                    Ec11Event::ClickedRotate(Ec11Direction::Cw, rotate_position, 1)
                }
                (Some(Ec11RotateStatus::CcwEnd), Some(Ec11PressStatus::Pressed)) => {
                    Ec11Event::ClickedRotate(Ec11Direction::Ccw, rotate_position, -1)
                }
                _ => Ec11Event::Empty,
            };

            if event != Ec11Event::Empty {
                event_tx.send(event).ok();
            }

            std::thread::sleep(Duration::from_millis(1));
        }
    });

    std::thread::spawn(move || {
        while let Ok(event) = event_rx.recv() {
            log::info!("ec11_event: {:?}", &event);
        }
    });
}
