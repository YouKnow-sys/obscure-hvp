pub fn bytes_sum(data: &[u8]) -> i32 {
    let chunks = data.chunks_exact(4);
    let reminder = chunks.remainder();
    let chunks_sum: i32 = chunks
        .map(|chunk| i32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
        .sum();
    let remainder_sum: i32 = reminder.iter().map(|&b| b as i32).sum();
    chunks_sum + remainder_sum
}

/*
based on c function:

int FUN_00542140(byte *param_1,uint param_2) {
  int iVar1;
  uint uVar2;

  iVar1 = 0;
  if (3 < param_2) {
    uVar2 = param_2 >> 2;
    do {
      iVar1 = iVar1 + *(int *)param_1;
      param_1 = param_1 + 4;
      param_2 = param_2 - 4;
      uVar2 = uVar2 - 1;
    } while (uVar2 != 0);
  }
  for (; param_2 != 0; param_2 = param_2 - 1) {
    iVar1 = iVar1 + (uint)*param_1;
    param_1 = param_1 + 1;
  }
  return iVar1;
}
*/
