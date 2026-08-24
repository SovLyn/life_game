use log::info;
use wasm_bindgen::prelude::*;

mod app;

fn main() -> Result<(), JsValue> {
    // 初始化日志，直接使用log的方法即可
    eframe::WebLogger::init(log::LevelFilter::Debug).ok();

    // 使用 wasm_bindgen_futures 在异步上下文中执行代码
    wasm_bindgen_futures::spawn_local(async {
        // 配置 eframe 的 Web 渲染选项
        let options = eframe::WebOptions::default();

        // 获取当前文档对象
        let document = web_sys::window()
            .expect("There is no window!") // 确保窗口对象存在
            .document()
            .expect("There is no document!"); // 确保文档对象存在

        // 获取主画布元素
        let canvas = document
            .get_element_by_id("main_port") // 通过 ID 获取元素
            .expect("Cannot find main port!") // 确保元素存在
            .dyn_into::<web_sys::HtmlCanvasElement>() // 转换为画布元素类型
            .expect("main port was not a HtmlCanvasElement"); // 确保转换成功

        let width = canvas.width() as usize;
        let height = canvas.height() as usize;
        info!("canvas size: {}x{}", width, height);

        // 启动 eframe Web 运行器
        let start_result = eframe::WebRunner::new()
            .start(
                canvas,  // 使用获取到的画布
                options, // 使用配置的选项
                Box::new(|cc| Ok(Box::new(app::LifeGameApp::new(cc)))),
            )
            .await;
        if let Err(e) = start_result {
            panic!("Failed to start eframe: {:?}", e);
        }
    });

    Ok(())
}
