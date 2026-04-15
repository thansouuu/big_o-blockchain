import * as anchor from "@coral-xyz/anchor";
import { 
  AddressLookupTableProgram, 
  PublicKey, 
  Transaction, 
  AddressLookupTableAccount 
} from "@solana/web3.js";

/**
 * Hàm hỗ trợ Khởi tạo và Mở rộng Address Lookup Table (ALT)
 */
export async function setupLookupTable(
  provider: anchor.AnchorProvider,
  addresses: PublicKey[]
): Promise<AddressLookupTableAccount> {
  const connection = provider.connection;

  // 1. Lấy slot hiện tại và tạo Instruction khởi tạo bảng
  const slot = await connection.getSlot("finalized");
  const [createIx, altAddress] = AddressLookupTableProgram.createLookupTable({
    authority: provider.publicKey,
    payer: provider.publicKey,
    recentSlot: slot,
  });

  // Gửi giao dịch tạo bảng
  const createTx = new Transaction().add(createIx);
  await provider.sendAndConfirm(createTx);
  console.log("✅ [ALT Helper] Đã tạo ALT tại:", altAddress.toBase58());

  // 2. Mở rộng bảng (Extend)
  // LƯU Ý TỪ GIÁO TRÌNH: Mỗi lần extend chỉ thêm được khoảng 20 địa chỉ. 
  // Thuật toán dưới đây sẽ tự động cắt mảng thành các chunk nhỏ.
  const CHUNK_SIZE = 20;
  for (let i = 0; i < addresses.length; i += CHUNK_SIZE) {
    const chunk = addresses.slice(i, i + CHUNK_SIZE);
    
    const extendIx = AddressLookupTableProgram.extendLookupTable({
      payer: provider.publicKey,
      authority: provider.publicKey,
      lookupTable: altAddress,
      addresses: chunk,
    });

    const extendTx = new Transaction().add(extendIx);
    await provider.sendAndConfirm(extendTx);
    console.log(`✅ [ALT Helper] Đã đẩy thành công ${chunk.length} địa chỉ vào bảng...`);
  }

  // 3. Xử lý thời gian chờ (Warmup)
  // Theo quy tắc, cần đợi ít nhất 1 slot để mạng lưới ghi nhận các địa chỉ mới
  console.log("⏳ [ALT Helper] Đang chờ 2 giây để ALT kích hoạt (warmup)...");
  await new Promise((resolve) => setTimeout(resolve, 2000));

  // 4. Fetch và trả về đối tượng ALT
  // Chúng ta trả về AddressLookupTableAccount thay vì chỉ PublicKey 
  // vì hàm compileToV0Message() sau này sẽ cần nguyên cái object này.
  const altAccountInfo = await connection.getAddressLookupTable(altAddress);
  
  if (!altAccountInfo.value) {
    throw new Error("❌ Không tìm thấy ALT hoặc chưa được kích hoạt!");
  }

  console.log("🎉 [ALT Helper] Hoàn tất! Tổng số địa chỉ trong bảng:", altAccountInfo.value.state.addresses.length);
  
  return altAccountInfo.value;
}