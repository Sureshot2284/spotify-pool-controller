# cargo build --release #Will create the macrogotchi-rs image in target/xtensa-esp32-espidf/release
espflash partition-table -o partition.bin  single-no-ota.csv --to-binary #Converts the csv file into an image that can be useb to create a single binary
espflash save-image -m dio --bootloader bootloader.bin --partition-table single-no-ota.csv --chip esp32 music-controller Music-controller.bin --merge -P #Merges all images into one binary that can be loaded onto esp32
# Bootloaders: https://github.com/esp-rs/espflash/tree/main/espflash/resources/bootloaders
espflash write-bin 0x0 .\Music-controller.bin #flash image to esp32