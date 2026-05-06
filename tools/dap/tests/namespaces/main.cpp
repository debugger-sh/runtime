#include <chrono>

namespace custom {
struct Widget {
  int value;
};

typedef Widget WidgetTypedef;
using WidgetUsing = Widget;
}  // namespace custom

int main() {
  std::chrono::time_point<std::chrono::system_clock, std::chrono::duration<long long, std::ratio<1, 1000000000>>> deep_chrono{};
  custom::Widget namespaced{1};
  custom::WidgetTypedef via_typedef{2};
  custom::WidgetUsing via_using{3};
  return namespaced.value + via_typedef.value + via_using.value + static_cast<int>(deep_chrono.time_since_epoch().count());
}
