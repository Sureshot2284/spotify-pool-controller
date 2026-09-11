esptool.py --chip esp32 merge_bin \
  -o merged-firmware.bin \
  --flash_mode dio \
  --flash_freq 40m \
  --flash_size keep \
  0x1000 bootloader.bin \
  0x9000 partition-table.bin \
  # 0xe000 boot.bin \
  0x10000 music-controller.bin