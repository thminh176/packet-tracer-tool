# Packet Tracer Tool

Bản 0.2.1 đang phát triển: sửa ánh xạ IPv6 forwarding, thêm IPv6 enable/SLAAC và bổ sung VLAN rõ ràng cho dot1Q. Thư viện có 48 nhóm lệnh giới hạn; chưa chứng nhận thực thi trên thiết bị Packet Tracer. Xem [bằng chứng kiểm thử](docs/TEST_RESULTS.md).

[English](README.md) · [Tải bản Windows](https://github.com/thminh176/packet-tracer-tool/releases)

**Ứng dụng desktop song ngữ độc lập hỗ trợ học, xem, đối chiếu và tìm lỗi trong bài lab Packet Tracer.** Giao diện có tên PKA Lab Inspector, viết bằng Rust / egui, chạy ngoại tuyến trên Windows x86_64, không cần Python.

Dự án **không liên kết và không được Cisco bảo trợ**. Cisco và Packet Tracer là nhãn hiệu của chủ sở hữu tương ứng. Repo không chứa chương trình Cisco, logo, giáo trình, bài thi riêng hay đáp án.

![Thư viện lệnh Windows với dữ liệu tổng hợp](docs/images/cisco-library-windows.png)

## Chức năng v0.2.0

- Thư viện lệnh Cisco song ngữ (Ctrl+7): 46 nhóm lệnh có tham số, chế độ cấu hình, điều kiện tiên quyết, hồ sơ thiết bị và liên kết tài liệu Cisco.
- Gợi ý gán VLAN từ yêu cầu cụ thể; không tự thêm chế độ cổng hay giả trạng thái đã xác minh.
- OSPF phải có process/area rõ ràng; đối chiếu tuyến chỉ dùng cấu hình hiện tại thuộc đúng thiết bị được nhận diện duy nhất.
- Phân tích cấu hình IOS theo ngữ cảnh; đối chiếu một số trường cổng và switching cụ thể. Tách số tiêu chí sinh được lệnh khỏi số tiêu chí đã đối chiếu. Xem [phạm vi lệnh](docs/CISCO_COMMAND_COVERAGE.md).

Các nhóm lệnh mới được kiểm thử tổng hợp; chưa xác nhận thực thi trên thiết bị Packet Tracer thật. Một số mô tả kỹ thuật vẫn dùng tiếng Anh.

- Mở tệp PKA/PKT thuộc định dạng được hỗ trợ ở chế độ chỉ đọc, xem XML.
- Hiện tiêu chí theo thiết bị/cổng, giá trị yêu cầu, điểm và bằng chứng đối chiếu.
- Phân biệt **Đạt / Chưa đạt / Chưa xác minh**; đánh dấu thủ công không đổi kết quả tự động.
- Tạo lệnh IOS khi có đủ dữ liệu cụ thể; hướng dẫn giao diện cho PC/máy chủ.
- Phân tích và chuẩn hóa định tuyến tĩnh IPv4/IPv6, tuyến mặc định (default route), cổng thoát (exit interface), khoảng cách quản trị (administrative distance) và định dạng chuỗi gạch nối của Packet Tracer.
- Xử lý khai báo mạng OSPF: tự động tính wildcard mask từ subnet mask, chuẩn hóa area (0 và 0.0.0.0), liên kết process ID và tạo khối lệnh `router ospf`.
- Chuẩn hóa tiền tố CIDR (`/prefix` và `-prefix`), mặt nạ mạng và wildcard mask với mức độ tin cậy và giải thích chẩn đoán rõ ràng.
- Nâng cấp chẩn đoán Auto Check khi đối chiếu `running-config`, giải thích rõ lý do khi chưa cấu hình tuyến hoặc thiếu tham số yêu cầu.
- Sao chép lệnh từng tiêu chí/thiết bị; xuất CSV, Markdown và tập lệnh. Đã gộp lệnh IP/mask trùng và tóm tắt mục chưa hỗ trợ.
- Chuyển Việt/Anh, lưu lựa chọn, thêm ngôn ngữ bằng JSON; đọc nền và cuộn dữ liệu lớn.

## Cài đặt

Tải `packet-tracer-tool-v0.2.0-windows-x64.zip` và `.sha256` tại [Releases](https://github.com/thminh176/packet-tracer-tool/releases). Kiểm tra checksum, giải nén toàn bộ và chạy `pka-lab-inspector.exe`. EXE chưa ký số. Giữ thư mục `locales` cạnh EXE để sửa hoặc thêm bản dịch.

Mở bài, đọc chẩn đoán, rồi nạp tệp cấu hình đã lưu để đối chiếu nếu cần. **Lệnh IOS phải dán vào Packet Tracer → đúng thiết bị → CLI; không chạy trong PowerShell.** PC/máy chủ dùng hướng dẫn giao diện. Ứng dụng không tự thực thi lệnh và không sửa tệp PKA/PKT gốc.

## Giới hạn

Ứng dụng chưa thay thế bộ chấm Cisco. Cách chọn tiêu chí dựa trên suy luận cấu trúc quan sát được. Các bản cấu hình nhúng không rõ vai trò, dữ liệu thiếu/mâu thuẫn, biến và bộ chấm tùy chỉnh vẫn là **Chưa xác minh**, không bị giả thành đạt hoặc trượt.

Định tuyến tĩnh và khai báo mạng OSPF được đối chiếu trực tiếp với running-config; tuy nhiên việc mô phỏng hội tụ định tuyến động, tính toán metric, kiểm tra toàn diện VLAN/ACL/NAT/DHCP, chấm bằng script, định dạng cũ và tích hợp trực tiếp Packet Tracer vẫn nằm ngoài phạm vi ngoại tuyến. Xem [bằng chứng kiểm thử](docs/TEST_RESULTS.md) và [tương thích](docs/COMPATIBILITY.md).

Bộ bản sao riêng được phép kiểm thử gồm 25 PKA và bảy PKT đều giải mã được; một mẫu khớp cấu trúc 32 tiêu chí/100 điểm/7 thiết bị. Đây là bằng chứng đọc/phân tích, chưa xác nhận thực thi lệnh hay kết quả chấm Cisco. Chưa xác nhận bài PKA thật có 126 tiêu chí. Mẫu tổng hợp 126 tiêu chí chỉ kiểm tra cấu trúc và một phần đối chiếu. Xem [kết quả tệp thật](docs/REAL_FILE_REVIEW.md).

Tiếng Việt (`vi-VN`) và English (`en-US`) dùng JSON với fallback tiếng Anh. Tên thiết bị, giá trị kỹ thuật và cú pháp IOS giữ nguyên. Báo cáo và một số thông báo chưa dịch vẫn dùng tiếng Anh. [Hướng dẫn thêm ngôn ngữ](docs/LOCALIZATION.md).

## Phát triển và đóng góp

Cài Rust stable, Visual Studio C++ Build Tools và Windows SDK, rồi chạy:

```powershell
cargo fmt --all -- --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked
cargo build --release --locked
```

Xem [CONTRIBUTING](CONTRIBUTING.md), [kiến trúc](docs/ARCHITECTURE.md), [bảo mật](SECURITY.md) và [changelog](CHANGELOG.md). Không gửi bài thi, giáo trình, đáp án, XML riêng hoặc mật khẩu vào issue.

## Ghi nhận và giấy phép

Mã ứng dụng gốc dùng MIT. Bộ giải mã dùng pktfile 0.2.2 của [TantiE100/pktctl](https://github.com/TantiE100/pktctl), MIT, kèm bản vá giới hạn giải nén; giữ nguyên bản quyền/giấy phép upstream. Xem [thư viện bên thứ ba](THIRD_PARTY_NOTICES.md), [kiểm toán giấy phép](docs/LICENSE_AUDIT.md) và [so sánh hệ sinh thái](docs/ECOSYSTEM_COMPARISON.md). Không sao chép hoặc chuyển ngữ triển khai có giấy phép hạn chế từ PKA Decipher.

Sau bản v0.1.1: mở rộng đối chiếu bằng mẫu hợp lệ, hỗ trợ thêm giao thức định tuyến (EIGRP/BGP), công cụ topology/subnet và nghiên cứu adapter pktctl tùy chọn ở chế độ chỉ đọc. Tính năng lớn làm trên nhánh riêng; không làm hỏng bản đã phát hành.

Nếu Windows báo thiếu `VCRUNTIME140.dll`, cài Microsoft Visual C++ Redistributable bản x64 từ liên kết Microsoft trong [hướng dẫn cài đặt](docs/INSTALLATION.md). Bộ cài runtime không nằm trong ZIP; ứng dụng dùng ngoại tuyến sau khi đã có đủ runtime hệ thống.
