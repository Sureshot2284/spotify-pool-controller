use esp_idf_svc::{
    nvs::EspNvs, 
    sys::EspError
};

use crate::NVS_DEFAULT;


pub fn save_data(key:&str,data:&str) -> Result<(),EspError>{
    let nvs_default_partition_lock = NVS_DEFAULT.lock().expect("NVS_DEFAULT is not held elsewhere");
    let nvs_default_partition = nvs_default_partition_lock.clone().expect("NVX_DEFAULT should be initialized");
    let test_namespace = "save_data";
    let mut nvs = match EspNvs::new(nvs_default_partition, test_namespace, true) {
        Ok(nvs) => nvs,
        Err(e) => panic!("Could't get namespace {:?}", e),
    };

    nvs.set_str(
        key,
        data,
    )
}

pub fn read_data<'a>(key:&str,buf:&'a mut [u8]) -> Result<Option<&'a str>,EspError> {
    let nvs_default_partition_lock = NVS_DEFAULT.lock().expect("NVS_DEFAULT is not held elsewhere");
    let nvs_default_partition = nvs_default_partition_lock.clone().expect("NVX_DEFAULT should be initialized");
    let test_namespace = "save_data";
    let nvs = match EspNvs::new(nvs_default_partition, test_namespace, true) {
        Ok(nvs) => nvs,
        Err(e) => panic!("Could't get namespace {:?}", e),
    };

    nvs.get_str(
        key,
        buf
    )
}

pub fn delete_data(key:&str) -> Result<bool,EspError> {
    let nvs_default_partition_lock = NVS_DEFAULT.lock().expect("NVS_DEFAULT is not held elsewhere");
    let nvs_default_partition = nvs_default_partition_lock.clone().expect("NVX_DEFAULT should be initialized");
    let test_namespace = "save_data";
    let mut nvs = match EspNvs::new(nvs_default_partition, test_namespace, true) {
        Ok(nvs) => nvs,
        Err(e) => panic!("Could't get namespace {:?}", e),
    };

    nvs.remove(key)
}

pub fn _data_is_set(key:&str) -> bool{
    let nvs_default_partition_lock = NVS_DEFAULT.lock().expect("NVS_DEFAULT is not held elsewhere");
    let nvs_default_partition = nvs_default_partition_lock.clone().expect("NVX_DEFAULT should be initialized");
    let test_namespace = "save_data";
    let nvs = match EspNvs::new(nvs_default_partition, test_namespace, true) {
        Ok(nvs) => nvs,
        Err(e) => panic!("Could't get namespace {:?}", e),
    };
    let mut buf = [0u8;1000];
    nvs.get_str(
        key,
        &mut buf
    )
    .expect("Data lookup should not error")
    .is_some()
}